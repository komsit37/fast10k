//! EDINET document reader for ZIP file content extraction and preview

use std::fs::File;
use std::io::Read;
use zip::ZipArchive;
use scraper::{Html, Selector};
use anyhow::{Result, Context};

/// Represents a section of an EDINET document
#[derive(Debug, Clone)]
pub struct DocumentSection {
    /// Section name/type (derived from filename)
    pub section_type: String,
    /// Raw filename within ZIP
    pub filename: String,
    /// Extracted text content (preview)
    pub content: String,
    /// Full content length before truncation
    pub full_length: usize,
}

/// File type mapping based on EDINET document structure
pub fn get_section_type(filename: &str) -> String {
    let base_name = filename
        .split('/')
        .last()
        .unwrap_or(filename)
        .to_string();
    
    if base_name.contains("0000000_header") {
        "Document Header".to_string()
    } else if base_name.contains("0101010_honbun") {
        "Business Overview".to_string()
    } else if base_name.contains("0102010_honbun") {
        "Risk Factors".to_string()
    } else if base_name.contains("0103010_honbun") {
        "Management Analysis".to_string()
    } else if base_name.contains("0104010_honbun") {
        "Financial Statements".to_string()
    } else if base_name.contains("0105000_honbun") {
        "Corporate Governance".to_string()
    } else if base_name.contains("0105010_honbun") {
        "Board of Directors".to_string()
    } else if base_name.contains("0105020_honbun") {
        "Executive Compensation".to_string()
    } else if base_name.contains("0105025_honbun") {
        "Stock Options".to_string()
    } else if base_name.contains("0105040_honbun") {
        "Accounting Auditor".to_string()
    } else if base_name.contains("0105050_honbun") {
        "Internal Control".to_string()
    } else if base_name.contains("0105100_honbun") {
        "Management Policy".to_string()
    } else if base_name.contains("0105110_honbun") {
        "Capital Structure".to_string()
    } else if base_name.contains("0105120_honbun") {
        "Dividend Policy".to_string()
    } else if base_name.contains("0105310_honbun") {
        "Related Party Transactions".to_string()
    } else if base_name.contains("0105320_honbun") {
        "Consolidated Subsidiaries".to_string()
    } else if base_name.contains("0105330_honbun") {
        "Business Segments".to_string()
    } else if base_name.contains("0106010_honbun") {
        "Research & Development".to_string()
    } else if base_name.contains("honbun") {
        "Content Section".to_string()
    } else if base_name.contains("fuzoku") {
        "Attachment".to_string()
    } else if base_name.ends_with(".xbrl") {
        "XBRL Data".to_string()
    } else {
        "Other".to_string()
    }
}

/// Extract text content from HTML using scraper
pub fn extract_text_from_html(html_content: &str, max_length: usize) -> Result<(String, usize)> {
    let document = Html::parse_document(html_content);
    
    // Try to find the main content div first
    let main_selector = Selector::parse("div#pageDIV, body").unwrap();
    let paragraph_selector = Selector::parse("p, div, td, th").unwrap();
    
    let mut text_content = String::new();
    
    // Look for main content area first
    if let Some(main_element) = document.select(&main_selector).next() {
        for element in main_element.select(&paragraph_selector) {
            let text = element.text().collect::<Vec<_>>().join(" ");
            let cleaned = text.trim();
            if !cleaned.is_empty() && cleaned.len() > 10 {
                text_content.push_str(cleaned);
                text_content.push('\n');
            }
        }
    }
    
    // Fallback: extract from all paragraphs if main content is empty
    if text_content.trim().is_empty() {
        for element in document.select(&paragraph_selector) {
            let text = element.text().collect::<Vec<_>>().join(" ");
            let cleaned = text.trim();
            if !cleaned.is_empty() && cleaned.len() > 10 {
                text_content.push_str(cleaned);
                text_content.push('\n');
            }
        }
    }
    
    let full_length = text_content.len();
    
    // Truncate if too long, ensuring we don't break UTF-8 character boundaries
    if text_content.len() > max_length {
        // Find the last character boundary within the limit
        let mut truncate_pos = max_length;
        while truncate_pos > 0 && !text_content.is_char_boundary(truncate_pos) {
            truncate_pos -= 1;
        }
        text_content.truncate(truncate_pos);
        text_content.push_str("...");
    }
    
    Ok((text_content, full_length))
}

/// Read and parse EDINET ZIP file contents
pub fn read_edinet_zip(
    zip_path: &str, 
    section_limit: usize, 
    preview_length: usize
) -> Result<Vec<DocumentSection>> {
    let file = File::open(zip_path)
        .with_context(|| format!("Failed to open ZIP file: {}", zip_path))?;
    
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP archive: {}", zip_path))?;
    
    let mut sections = Vec::new();
    let mut processed_count = 0;
    
    // Collect and sort file entries - prioritize main content files
    let mut file_entries: Vec<(usize, String)> = (0..archive.len())
        .map(|i| {
            let file = archive.by_index(i).unwrap();
            (i, file.name().to_string())
        })
        .collect();
    
    // Sort to prioritize important sections
    file_entries.sort_by(|a, b| {
        let priority_a = get_file_priority(&a.1);
        let priority_b = get_file_priority(&b.1);
        priority_a.cmp(&priority_b)
    });
    
    for (index, filename) in file_entries {
        if processed_count >= section_limit {
            break;
        }
        
        // Skip non-content files
        if filename.contains("fuzoku/") || 
           (!filename.contains("honbun") && !filename.contains("header") && !filename.ends_with(".xbrl")) {
            continue;
        }
        
        let mut file = archive.by_index(index)
            .with_context(|| format!("Failed to read file from ZIP: {}", filename))?;
        
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(_) => {
                let section_type = get_section_type(&filename);
                
                let (extracted_text, full_length) = if filename.ends_with(".htm") {
                    extract_text_from_html(&contents, preview_length)?
                } else if filename.ends_with(".xbrl") {
                    // For XBRL files, just show a sample of the raw content
                    let preview = if contents.len() > preview_length {
                        let mut truncate_pos = preview_length;
                        while truncate_pos > 0 && !contents.is_char_boundary(truncate_pos) {
                            truncate_pos -= 1;
                        }
                        format!("{}...", &contents[..truncate_pos])
                    } else {
                        contents.clone()
                    };
                    (preview, contents.len())
                } else {
                    // For other files, show raw content preview
                    let preview = if contents.len() > preview_length {
                        let mut truncate_pos = preview_length;
                        while truncate_pos > 0 && !contents.is_char_boundary(truncate_pos) {
                            truncate_pos -= 1;
                        }
                        format!("{}...", &contents[..truncate_pos])
                    } else {
                        contents.clone()
                    };
                    (preview, contents.len())
                };
                
                sections.push(DocumentSection {
                    section_type,
                    filename: filename.clone(),
                    content: extracted_text,
                    full_length,
                });
                
                processed_count += 1;
            }
            Err(_) => {
                // Skip binary files or files that can't be read as text
                continue;
            }
        }
    }
    
    Ok(sections)
}

/// Get file priority for sorting (lower number = higher priority)
fn get_file_priority(filename: &str) -> u32 {
    if filename.contains("0000000_header") { 0 }
    else if filename.contains("0101010_honbun") { 1 }
    else if filename.contains("0102010_honbun") { 2 }
    else if filename.contains("0103010_honbun") { 3 }
    else if filename.contains("0104010_honbun") { 4 }
    else if filename.contains("0105100_honbun") { 5 }
    else if filename.contains("honbun") { 10 }
    else if filename.ends_with(".xbrl") { 20 }
    else { 99 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_type_detection() {
        assert_eq!(get_section_type("0000000_header_test.htm"), "Document Header");
        assert_eq!(get_section_type("0101010_honbun_test.htm"), "Business Overview");
        assert_eq!(get_section_type("0104010_honbun_test.htm"), "Financial Statements");
        assert_eq!(get_section_type("fuzoku/image.gif"), "Attachment");
        assert_eq!(get_section_type("test.xbrl"), "XBRL Data");
    }

    #[test]
    fn test_file_priority() {
        assert!(get_file_priority("0000000_header.htm") < get_file_priority("0101010_honbun.htm"));
        assert!(get_file_priority("0101010_honbun.htm") < get_file_priority("0104010_honbun.htm"));
        assert!(get_file_priority("test.xbrl") < get_file_priority("fuzoku/image.gif"));
    }
}