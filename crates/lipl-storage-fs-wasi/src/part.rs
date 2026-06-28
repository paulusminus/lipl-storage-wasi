use crate::bindings::exports::pm::lipl_core::types::Error;

#[allow(dead_code)]
pub fn to_parts<S>(s: S) -> Vec<Vec<String>>
where
    S: ToString,
{
    s.to_string()
        .split('\n')
        .map(str::trim)
        .map(String::from)
        .collect::<Vec<_>>()
        .split(String::is_empty)
        .map(|part| part.to_vec())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
}

#[allow(dead_code)]
pub fn to_text(parts: &[Vec<String>]) -> String {
    parts
        .iter()
        .map(|part| part.join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[allow(dead_code)]
pub fn extract_delimited_frontmatter<'a>(content: &'a str) -> Result<(&'a str, &'a str), Error> {
    let indexes = content.split("+++\n").collect::<Vec<_>>();
    if indexes.len() == 3 {
        Ok((&indexes[1], &indexes[2]))
    } else {
        Err(Error::NotFound("".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_delimited_frontmatter, to_parts};

    #[test]
    fn test_to_part() {
        let s = "line1\nline2\n \n\n  part2\n\npart3\n";
        let parts = to_parts(s);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], vec!["line1", "line2"]);
        assert_eq!(parts[1], vec!["part2"]);
        assert_eq!(parts[2], vec!["part3"]);
    }

    #[test]
    fn test_extract_delimited_frontmatter() {
        let content =
            "+++\ntitle: My Title\ndate: 2023-01-01\n+++\nEr is er één jarig\nhoera, hoera";
        let frontmatter = extract_delimited_frontmatter(content);
        assert_eq!(
            frontmatter.unwrap(),
            (
                "title: My Title\ndate: 2023-01-01\n",
                "Er is er één jarig\nhoera, hoera"
            )
        );
    }
}
