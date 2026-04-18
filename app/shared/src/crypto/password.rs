pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(password, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let pw = "MySecureP@ss123";
        let hash = hash_password(pw).expect("hash should succeed");
        assert!(
            verify_password(pw, &hash).expect("verify should succeed"),
            "correct password should verify"
        );
    }

    #[test]
    fn wrong_password_returns_false() {
        let hash = hash_password("correct-password").expect("hash should succeed");
        let result = verify_password("wrong-password", &hash).expect("verify should succeed");
        assert!(!result, "wrong password should return false");
    }
}
