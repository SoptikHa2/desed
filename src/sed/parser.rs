use anyhow::Result;
use crate::sed::debugger::DebuggingState;

/// Parsed anotation received from GNU sed
pub struct SedAnnotation<'a> {
    /// Source code of sed program, cleaned up.
    /// That means one command per line, without comments. This is what gets displayed to user.
    pub program_source: &'a str,
    /// All states sed ever was in. This is generally one state per instruction. Capturing all of it and storing it
    /// allows us to time-travel during debugging.
    pub states: Vec<DebuggingState<'a>>,
    /// Optionally, sed might've printed something after the last instruction. If it was the case, we show it up here. This is
    /// generally output of the sed script as the user sees it.
    pub last_output: Option<&'a str>,
}

const SED_ANNOTATION_INSTRUCTION_COUNT: usize = 7;
const SED_ANNOTATION_INSTRUCTIONS: [&str; SED_ANNOTATION_INSTRUCTION_COUNT] = [
    "\nSED PROGRAM:\n",
    "\nINPUT:   ",
    "\nPATTERN: ",
    "\nCOMMAND:               ",
    "\nHOLD:    ",
    "\nMATCHED REGEX REGISTERS\n",
    "\nEND-OF-CYCLE:\n"
];

pub struct SedAnnotationParser {}
impl SedAnnotationParser {
    pub fn parse_sed_debug_annotation<'a>(input: String) -> Result<SedAnnotation<'a>> {
        unimplemented!();
    }

    /// Traverse input text until a next instruction, which has to be on a newline.
    /// 
    /// All instructions are set to begin with a newline. However, there is a flag: ignore_preceding_newline,
    /// (this should be used on *first* parse of the sed output), which suppresses the need to have a newline
    /// at the beginning.
    /// 
    /// Output is a tuple consisting of:
    /// 1. String from start of input until the next instruction (EXCLUDING the instruction)
    /// 2. The instruction matched
    /// 3. The string AFTER instruction (EXCLUDING the instruction itself and EXCLUDING newline at end of the instruction, if any)
    fn get_text_until_next_instruction<'a>(input: &'a str, ignore_preceding_newline: bool) -> Result<(&'a str, Option<&'a str>, &'a str)> {
        // This represents where the selected sed annotation instruction starts
        let mut pattern_end: usize = 0;
        // This represents how many characters of individual sed annotation instructions were actually matched
        let mut correct_characters_per_annotation: [usize; SED_ANNOTATION_INSTRUCTION_COUNT] = 
            [ if ignore_preceding_newline { 1 } else { 0 }; SED_ANNOTATION_INSTRUCTION_COUNT];
        let mut selected_annotation: Option<&str> = None;

        // For each character in the input...
        'outer: for (input_char_idx, input_char) in input.chars().enumerate() {
            // Check if it matches any sed instruction
            for (sed_instruction_idx, sed_instruction) in SED_ANNOTATION_INSTRUCTIONS.iter().enumerate() {
                // Unwrap: assuming there are no zero-size annotation instructions, this function exits the moment any
                // instruction reaches the end, so this code cannot run if it were to exceed instruction length
                if sed_instruction.chars().skip(correct_characters_per_annotation[sed_instruction_idx]).next().unwrap() == input_char {
                    correct_characters_per_annotation[sed_instruction_idx] += 1;
                    if correct_characters_per_annotation[sed_instruction_idx] == sed_instruction.len() {
                        selected_annotation = Some(&sed_instruction[1..]);
                        pattern_end = input_char_idx + 1;
                        break 'outer;
                    }
                } else {
                    // If this fails, check if at least the first character matches
                    // Unwrap: there are no zero-sized instructions, and we check just the first character.
                    if sed_instruction.chars().next().unwrap() == input_char {
                        correct_characters_per_annotation[sed_instruction_idx] = 1;
                    } else {
                        correct_characters_per_annotation[sed_instruction_idx] = 0;
                    }
                }
            }
        }

        if let Some(selected_annotation) = selected_annotation {
            let pattern_start = pattern_end - selected_annotation.len();
            Ok((&input[..pattern_start], Some(selected_annotation), &input[pattern_end..]))
        } else {
            Ok(("", None, input))
        }

    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn split_text_by_instructions() {
        let input = "SED PROGRAM:\nabc\ndef\nghiINPUT:   WRONG_INPUT\nINPUT:   correct\ninput\nEND-OF-CYCLE:\nend";
        let (before, instr, after) = SedAnnotationParser::get_text_until_next_instruction(&input, true).unwrap();
        assert_eq!("", before, "Before should be empty, as literally the first thing in test string is instruction.");
        assert_eq!(Some("SED PROGRAM:\n"), instr);
        assert_eq!("abc\ndef\nghiINPUT:   WRONG_INPUT\nINPUT:   correct\ninput\nEND-OF-CYCLE:\nend", after);

        let (before, instr, after) = SedAnnotationParser::get_text_until_next_instruction(&after, false).unwrap();
        assert_eq!("abc\ndef\nghiINPUT:   WRONG_INPUT\n", before, "Before should contain string (with the most interesting part being ghiINPUT:     ) to test that
        identifiers are only matched on newlines or string start. If it doesn't have the INPUT:     , then the annotation instruction was parsed as instruction
        even if it was not on new line itself.");
        assert_eq!(Some("INPUT:   "), instr);
        assert_eq!("correct\ninput\nEND-OF-CYCLE:\nend", after);

        let (before, instr, after) = SedAnnotationParser::get_text_until_next_instruction(&after, false).unwrap();
        assert_eq!("correct\ninput\n", before);
        assert_eq!(Some("END-OF-CYCLE:\n"), instr);
        assert_eq!("end", after);

        let (before, instr, after) = SedAnnotationParser::get_text_until_next_instruction(&after, false).unwrap();
        assert_eq!("", before);
        assert_eq!(None, instr);
        assert_eq!("end", after);

        let (before, instr, after) = SedAnnotationParser::get_text_until_next_instruction(&after, false).unwrap();
        assert_eq!("", before);
        assert_eq!(None, instr);
        assert_eq!("end", after);
    }
}
