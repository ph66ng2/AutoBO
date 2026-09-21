/// Validate Brazilian CPF (Cadastro de Pessoa Física).
/// Returns true if the CPF has valid check digits.
pub fn validar_cpf(cpf: &str) -> bool {
    let cpf: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
    if cpf.len() != 11 {
        return false;
    }
    // Reject known invalid patterns (all same digit)
    if cpf.chars().all(|c| c == cpf.chars().next().unwrap()) {
        return false;
    }
    // Validate check digits
    let digits: Vec<u32> = cpf.chars().filter_map(|c| c.to_digit(10)).collect();
    // First check digit
    let sum: u32 = (0..9).map(|i| digits[i] * (10 - i as u32)).sum();
    let d1 = if sum * 10 % 11 >= 10 { 0 } else { sum * 10 % 11 };
    if d1 != digits[9] {
        return false;
    }
    // Second check digit
    let sum: u32 = (0..10).map(|i| digits[i] * (11 - i as u32)).sum();
    let d2 = if sum * 10 % 11 >= 10 { 0 } else { sum * 10 % 11 };
    d2 == digits[10]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cpf() {
        assert!(validar_cpf("52998224725"));
        assert!(validar_cpf("529.982.247-25"));
    }

    #[test]
    fn test_invalid_cpf_all_same() {
        assert!(!validar_cpf("11111111111"));
        assert!(!validar_cpf("00000000000"));
        assert!(!validar_cpf("99999999999"));
    }

    #[test]
    fn test_invalid_cpf_wrong_digits() {
        assert!(!validar_cpf("12345678901"));
        assert!(!validar_cpf("52998224726"));
    }

    #[test]
    fn test_invalid_cpf_too_short() {
        assert!(!validar_cpf("1234567890"));
    }

    #[test]
    fn test_invalid_cpf_too_long() {
        assert!(!validar_cpf("123456789012"));
    }

    #[test]
    fn test_invalid_cpf_empty() {
        assert!(!validar_cpf(""));
    }

    #[test]
    fn test_valid_cpf_with_formatting() {
        assert!(validar_cpf("  529.982.247-25  "));
        assert!(validar_cpf("529-982-247.25"));
    }
}
