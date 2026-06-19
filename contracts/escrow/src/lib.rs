(the file content from the current lib.rs with the following appended at the end)

// --- BEGIN: categorized sub-enums for Error (preserve public numeric codes via conversions)

/// For internal use: grouped escrow-related errors.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum EscrowError {
    EscrowNotFound,
    InvalidStatus,
    AlreadyProcessed,
    Unauthorized,
    ReleaseNotYetAvailable,
    NotDisputed,
    TimeoutNotReached,
    ReleaseOnHoldPeriod,
    InvalidVestingSchedule,
    CliffPeriodNotPassed,
    MilestoneAlreadyReleased,
    DuplicateApproval,
    ApprovalsThresholdNotMet,
    MultiSigNotInitialized,
    NotAnAdmin,
    AlreadyApproved,
    ActionNotReady,
    ContractPaused,
    TransferNotAllowed,
    SameBeneficiary,
    ConditionalEscrowNotFound,
    ConditionAlreadyEvaluated,
    InvalidMerkleProof,
    RootAlreadyCommitted,
    BatchReleaseSizeLimitExceeded,
    EvidenceDeadlinePassed,
    MaxHierarchyDepth,
    ParentEscrowNotFound,
    ChildrenNotResolved,
}

/// Observer-related errors.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ObserverError {
    ObserverAlreadyAdded,
    ObserverNotFound,
}

/// Migration-related errors.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum MigrationError {
    MigrationNotStarted,
    AlreadyMigrated,
}

/// Participant & template related errors.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ParticipantError {
    ParticipantNotFound,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum TemplateError {
    TemplateNotFound,
    TemplateInactive,
}

/// Sub-account related errors.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum SubAccountError {
    SubAccountNotFound,
    SubAccountAlreadyReleased,
    SubAccountFundingExceedsEscrow,
}

/// Swap-related errors.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum SwapError {
    SwapConfigNotFound,
    SwapOutputBelowMinimum,
    SwapAlreadyExecuted,
}

// Conversion impls that preserve the public Error numeric discriminants by
// mapping the internal grouped enums back to the original `Error` variants.
impl From<EscrowError> for Error {
    fn from(e: EscrowError) -> Self {
        match e {
            EscrowError::EscrowNotFound => Error::EscrowNotFound,
            EscrowError::InvalidStatus => Error::InvalidStatus,
            EscrowError::AlreadyProcessed => Error::AlreadyProcessed,
            EscrowError::Unauthorized => Error::Unauthorized,
            EscrowError::ReleaseNotYetAvailable => Error::ReleaseNotYetAvailable,
            EscrowError::NotDisputed => Error::NotDisputed,
            EscrowError::TimeoutNotReached => Error::TimeoutNotReached,
            EscrowError::ReleaseOnHoldPeriod => Error::ReleaseOnHoldPeriod,
            EscrowError::InvalidVestingSchedule => Error::InvalidVestingSchedule,
            EscrowError::CliffPeriodNotPassed => Error::CliffPeriodNotPassed,
            EscrowError::MilestoneAlreadyReleased => Error::MilestoneAlreadyReleased,
            EscrowError::DuplicateApproval => Error::DuplicateApproval,
            EscrowError::ApprovalsThresholdNotMet => Error::ApprovalsThresholdNotMet,
            EscrowError::MultiSigNotInitialized => Error::MultiSigNotInitialized,
            EscrowError::NotAnAdmin => Error::NotAnAdmin,
            EscrowError::AlreadyApproved => Error::AlreadyApproved,
            EscrowError::ActionNotReady => Error::ActionNotReady,
            EscrowError::ContractPaused => Error::ContractPaused,
            EscrowError::TransferNotAllowed => Error::TransferNotAllowed,
            EscrowError::SameBeneficiary => Error::SameBeneficiary,
            EscrowError::ConditionalEscrowNotFound => Error::ConditionalEscrowNotFound,
            EscrowError::ConditionAlreadyEvaluated => Error::ConditionAlreadyEvaluated,
            EscrowError::InvalidMerkleProof => Error::InvalidMerkleProof,
            EscrowError::RootAlreadyCommitted => Error::RootAlreadyCommitted,
            EscrowError::BatchReleaseSizeLimitExceeded => Error::BatchReleaseSizeLimitExceeded,
            EscrowError::EvidenceDeadlinePassed => Error::EvidenceDeadlinePassed,
            EscrowError::MaxHierarchyDepth => Error::MaxHierarchyDepth,
            EscrowError::ParentEscrowNotFound => Error::ParentEscrowNotFound,
            EscrowError::ChildrenNotResolved => Error::ChildrenNotResolved,
        }
    }
}

impl From<ObserverError> for Error {
    fn from(e: ObserverError) -> Self {
        match e {
            ObserverError::ObserverAlreadyAdded => Error::ObserverAlreadyAdded,
            ObserverError::ObserverNotFound => Error::ObserverNotFound,
        }
    }
}

impl From<MigrationError> for Error {
    fn from(e: MigrationError) -> Self {
        match e {
            MigrationError::MigrationNotStarted => Error::MigrationNotStarted,
            MigrationError::AlreadyMigrated => Error::AlreadyMigrated,
        }
    }
}

impl From<ParticipantError> for Error {
    fn from(e: ParticipantError) -> Self {
        match e {
            ParticipantError::ParticipantNotFound => Error::ParticipantNotFound,
        }
    }
}

impl From<TemplateError> for Error {
    fn from(e: TemplateError) -> Self {
        match e {
            TemplateError::TemplateNotFound => Error::TemplateNotFound,
            TemplateError::TemplateInactive => Error::TemplateInactive,
        }
    }
}

impl From<SubAccountError> for Error {
    fn from(e: SubAccountError) -> Self {
        match e {
            SubAccountError::SubAccountNotFound => Error::SubAccountNotFound,
            SubAccountError::SubAccountAlreadyReleased => Error::SubAccountAlreadyReleased,
            SubAccountError::SubAccountFundingExceedsEscrow => Error::SubAccountFundingExceedsEscrow,
        }
    }
}

impl From<SwapError> for Error {
    fn from(e: SwapError) -> Self {
        match e {
            SwapError::SwapConfigNotFound => Error::SwapConfigNotFound,
            SwapError::SwapOutputBelowMinimum => Error::SwapOutputBelowMinimum,
            SwapError::SwapAlreadyExecuted => Error::SwapAlreadyExecuted,
        }
    }
}

// --- END: categorized sub-enums for Error
