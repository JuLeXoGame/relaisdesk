use std::io::{self, BufRead, Write};

pub fn confirm_overwrite(
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<bool> {
    loop {
        write!(output, "Overwrite the existing file? [y/N] ")?;
        output.flush()?;
        let mut answer = String::new();
        if input.read_line(&mut answer)? == 0 {
            return Ok(false);
        }
        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "" | "n" | "no" => return Ok(false),
            _ => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overwrite_requires_explicit_confirmation() {
        for (input, expected) in [
            ("", false),
            ("\n", false),
            ("N\n", false),
            ("no\n", false),
            ("y\n", true),
            (" YES \r\n", true),
            ("invalid\n", false),
            ("invalid\nyes\n", true),
        ] {
            assert_eq!(
                confirm_overwrite(&mut input.as_bytes(), &mut Vec::new()).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn read_failure_does_not_authorize_overwrite() {
        let mut input = io::Cursor::new([0xff, 0xfe, b'\n']);
        assert!(confirm_overwrite(&mut input, &mut Vec::new()).is_err());
    }
}
