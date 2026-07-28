use crate::{FloatDomain, HybridDomain, SetIntervalDomain};
use propaga_core::DomainView;

/// Tagged domain stored in the propagation engine.
#[derive(Clone, Debug, PartialEq)]
pub enum AnyDomain {
    Int(HybridDomain),
    Set(SetIntervalDomain),
    Float(FloatDomain),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainKind {
    Int,
    Set,
    Float,
}

impl AnyDomain {
    #[must_use]
    pub fn kind(&self) -> DomainKind {
        match self {
            Self::Int(_) => DomainKind::Int,
            Self::Set(_) => DomainKind::Set,
            Self::Float(_) => DomainKind::Float,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Int(domain) => domain.is_empty(),
            Self::Set(domain) => domain.is_empty(),
            Self::Float(domain) => domain.is_empty(),
        }
    }

    #[must_use]
    pub fn is_fixed(&self) -> bool {
        match self {
            Self::Int(domain) => domain.is_fixed(),
            Self::Set(domain) => domain.is_fixed(),
            Self::Float(domain) => domain.is_fixed(),
        }
    }

    #[must_use]
    pub fn as_int(&self) -> Option<&HybridDomain> {
        match self {
            Self::Int(domain) => Some(domain),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_set(&self) -> Option<&SetIntervalDomain> {
        match self {
            Self::Set(domain) => Some(domain),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_float(&self) -> Option<&FloatDomain> {
        match self {
            Self::Float(domain) => Some(domain),
            _ => None,
        }
    }

    #[must_use]
    pub fn size(&self) -> usize {
        match self {
            Self::Int(domain) => domain.size(),
            Self::Set(domain) => domain.size(),
            Self::Float(domain) => domain.size(),
        }
    }
}

impl From<HybridDomain> for AnyDomain {
    fn from(domain: HybridDomain) -> Self {
        Self::Int(domain)
    }
}

impl From<SetIntervalDomain> for AnyDomain {
    fn from(domain: SetIntervalDomain) -> Self {
        Self::Set(domain)
    }
}

impl From<FloatDomain> for AnyDomain {
    fn from(domain: FloatDomain) -> Self {
        Self::Float(domain)
    }
}

impl From<crate::IntervalDomain> for AnyDomain {
    fn from(domain: crate::IntervalDomain) -> Self {
        Self::Int(domain.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_domain_converts_to_int() {
        let domain = AnyDomain::from(HybridDomain::new(1, 5));
        assert_eq!(domain.kind(), DomainKind::Int);
    }
}
