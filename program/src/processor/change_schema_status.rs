use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};
use pinocchio_log::log;

use crate::{
    error::AttestationServiceError,
    processor::{verify_owner_mutability, verify_signer},
    state::{discriminator::AccountSerialize, Credential, Schema},
};

#[inline(always)]
pub fn process_change_schema_status(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let [authority_info, credential_info, schema_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate: authority should have signed
    verify_signer(authority_info, false)?;

    // Verify program ownership, mutability and PDAs.
    verify_owner_mutability(credential_info, program_id, false)?;
    verify_owner_mutability(schema_info, program_id, true)?;

    // Read is_paused from instruction data.
    let is_paused = instruction_data
        .get(0)
        .ok_or(ProgramError::InvalidInstructionData)?
        .eq(&1);

    let credential = &Credential::try_from_bytes(&credential_info.try_borrow_data()?)?;
    credential.verify_pda(credential_info, program_id)?;

    // Verify signer matches credential authority.
    if credential.authority.ne(authority_info.key()) {
        return Err(ProgramError::IncorrectAuthority);
    }

    let mut schema_data = schema_info.try_borrow_mut_data()?;
    let mut schema = Schema::try_from_bytes(&schema_data)?;
    schema.verify_pda(schema_info, program_id)?;

    // Verify that schema is under the same credential.
    if schema.credential.ne(credential_info.key()) {
        return Err(AttestationServiceError::InvalidSchema.into());
    }

    schema.is_paused = is_paused;
    log!("Setting schema's is_paused to: {}", is_paused as u8);
    schema_data.copy_from_slice(&schema.to_bytes());

    Ok(())
}
