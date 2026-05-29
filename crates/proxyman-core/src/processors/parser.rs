pub trait ProcessorRuleParser {
    type Rule;
    fn parse_rule(content: &str) -> Self::Rule;
}
