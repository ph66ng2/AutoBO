/// Validate Brazilian CNPJ (Cadastro Nacional de Pessoa Jurídica).
/// Returns true if the CNPJ has valid check digits.
pub fn validar_cnpj(cnpj: &str) -> bool {
    let cnpj: String = cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
    if cnpj.len() != 14 {
        return false;
    }
    if cnpj.chars().all(|c| c == cnpj.chars().next().unwrap()) {
        return false;
    }
    let digits: Vec<u32> = cnpj.chars().filter_map(|c| c.to_digit(10)).collect();
    let weights1 = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let weights2 = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum: u32 = (0..12).map(|i| digits[i] * weights1[i]).sum();
    let d1 = if sum % 11 < 2 { 0 } else { 11 - sum % 11 };
    if d1 != digits[12] {
        return false;
    }
    let sum: u32 = (0..13).map(|i| digits[i] * weights2[i]).sum();
    let d2 = if sum % 11 < 2 { 0 } else { 11 - sum % 11 };
    d2 == digits[13]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cnpj() {
        assert!(validar_cnpj("11222333000181"));
        assert!(validar_cnpj("11.222.333/0001-81"));
    }

    #[test]
    fn test_invalid_cnpj_all_same() {
        assert!(!validar_cnpj("11111111111111"));
        assert!(!validar_cnpj("00000000000000"));
        assert!(!validar_cnpj("99999999999999"));
    }

    #[test]
    fn test_invalid_cnpj_wrong_digits() {
        assert!(!validar_cnpj("11222333000182"));
        assert!(!validar_cnpj("12345678000100"));
    }

    #[test]
    fn test_invalid_cnpj_too_short() {
        assert!(!validar_cnpj("1234567800019"));
    }

    #[test]
    fn test_invalid_cnpj_too_long() {
        assert!(!validar_cnpj("123456780001951"));
    }

    #[test]
    fn test_invalid_cnpj_empty() {
        assert!(!validar_cnpj(""));
    }

    #[test]
    fn test_valid_cnpj_with_formatting() {
        assert!(validar_cnpj("  11.222.333/0001-81  "));
        assert!(validar_cnpj("11-222-333/0001.81"));
    }
}
