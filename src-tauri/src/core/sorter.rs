use std::cmp::Ordering;

use crate::core::models::FileEntry;

/// Compare two strings using natural sort order.
/// Numeric segments are compared as numbers, text segments lexicographically.
pub fn natural_sort_compare(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&a_ch), Some(&b_ch)) => {
                if a_ch.is_ascii_digit() && b_ch.is_ascii_digit() {
                    // Extract and compare numeric segments
                    let (a_num, a_rest) = extract_number(&a[a.len() - a_chars.clone().count()..]);
                    let (b_num, b_rest) = extract_number(&b[b.len() - b_chars.clone().count()..]);

                    match a_num.cmp(&b_num) {
                        Ordering::Equal => {
                            // Continue after the number
                            a_chars = a_rest.chars().peekable();
                            b_chars = b_rest.chars().peekable();
                        }
                        other => return other,
                    }
                } else {
                    // Compare as lowercase characters
                    let a_lower = a_ch.to_ascii_lowercase();
                    let b_lower = b_ch.to_ascii_lowercase();
                    match a_lower.cmp(&b_lower) {
                        Ordering::Equal => {
                            a_chars.next();
                            b_chars.next();
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

fn extract_number(s: &str) -> (u64, &str) {
    let mut num_str = String::new();
    let mut rest_start = 0;

    for (i, ch) in s.char_indices() {
        if ch.is_ascii_digit() {
            num_str.push(ch);
            rest_start = i + ch.len_utf8();
        } else {
            break;
        }
    }

    if num_str.is_empty() {
        (0, s)
    } else {
        let num = num_str.parse::<u64>().unwrap_or(u64::MAX);
        (num, &s[rest_start..])
    }
}

/// Sort files by natural order of their file names.
pub fn natural_sort_files(files: &mut [FileEntry]) {
    files.sort_by(|a, b| natural_sort_compare(&a.file_name, &b.file_name));
}

/// Sort paths by natural order of their file names.
pub fn natural_sort_paths(paths: &mut [std::path::PathBuf]) {
    paths.sort_by(|a, b| {
        let name_a = a
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let name_b = b
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        natural_sort_compare(&name_a, &name_b)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_natural_sort_numbers_only() {
        let mut names: Vec<&str> = vec!["10.jpg", "2.jpg", "1.jpg", "20.jpg", "100.jpg"];
        names.sort_by(|a, b| natural_sort_compare(a, b));
        assert_eq!(names, vec!["1.jpg", "2.jpg", "10.jpg", "20.jpg", "100.jpg"]);
    }

    #[test]
    fn test_natural_sort_mixed() {
        let mut names: Vec<&str> = vec!["img_10.png", "img_2.png", "img_1.png"];
        names.sort_by(|a, b| natural_sort_compare(a, b));
        assert_eq!(
            names,
            vec!["img_1.png", "img_2.png", "img_10.png"]
        );
    }

    #[test]
    fn test_natural_sort_text_vs_number() {
        let mut names: Vec<&str> = vec!["apple.jpg", "10.jpg", "banana.png", "2.txt"];
        names.sort_by(|a, b| natural_sort_compare(a, b));
        assert_eq!(
            names,
            vec!["2.txt", "10.jpg", "apple.jpg", "banana.png"]
        );
    }

    #[test]
    fn test_natural_sort_case_insensitive() {
        let mut names: Vec<&str> = vec!["B.jpg", "a.jpg", "C.png"];
        names.sort_by(|a, b| natural_sort_compare(a, b));
        assert_eq!(names, vec!["a.jpg", "B.jpg", "C.png"]);
    }

    #[test]
    fn test_natural_sort_empty_strings() {
        let mut names: Vec<&str> = vec!["", "a", ""];
        names.sort_by(|a, b| natural_sort_compare(a, b));
        assert_eq!(names, vec!["", "", "a"]);
    }

    #[test]
    fn test_natural_sort_chinese() {
        let mut names: Vec<&str> = vec!["照片2.jpg", "照片10.jpg", "照片1.jpg"];
        names.sort_by(|a, b| natural_sort_compare(a, b));
        assert_eq!(
            names,
            vec!["照片1.jpg", "照片2.jpg", "照片10.jpg"]
        );
    }

    #[test]
    fn test_natural_sort_paths() {
        let mut paths: Vec<PathBuf> = vec![
            PathBuf::from("/dir/10.txt"),
            PathBuf::from("/dir/2.txt"),
            PathBuf::from("/dir/1.txt"),
        ];
        natural_sort_paths(&mut paths);
        let names: Vec<&str> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, vec!["1.txt", "2.txt", "10.txt"]);
    }
}
