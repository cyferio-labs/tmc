mod call;
mod event;
mod genesis;
#[cfg(feature = "native")]
mod query;
pub use call::CallMessage;
pub use event::Event;
#[cfg(feature = "native")]
pub use query::*;
use serde::{Deserialize, Serialize};
use sov_modules_api::{
    CallResponse, Context, Error, GenesisState, Module, ModuleId, ModuleInfo, ModuleRestApi, Spec,
    StateValue, TxState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExampleModuleConfig {}

/// A new module:
/// - Must derive `ModuleInfo`
/// - Must contain `[id]` field
/// - Can contain any number of ` #[state]` or `[module]` fields
/// - Can derive ModuleRestApi to automatically generate Rest API endpoints
#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct ExampleModule<S: Spec> {
    /// Id of the module.
    #[id]
    pub id: ModuleId,

    /// Some value kept in the state.
    #[state]
    pub value: StateValue<u32>,

    /// Reference to the Bank module.
    #[module]
    pub(crate) _bank: sov_bank::Bank<S>,
}

impl<S: Spec> Module for ExampleModule<S> {
    type Spec = S;

    type Config = ExampleModuleConfig;

    type CallMessage = call::CallMessage;

    type Event = Event;

    fn genesis(
        &self,
        config: &Self::Config,
        state: &mut impl GenesisState<S>,
    ) -> Result<(), Error> {
        // The initialization logic
        Ok(self.init_module(config, state)?)
    }

    fn call(
        &self,
        msg: Self::CallMessage,
        context: &Context<Self::Spec>,
        state: &mut impl TxState<S>,
    ) -> Result<CallResponse, Error> {
        match msg {
            call::CallMessage::SetValue(new_value) => {
                Ok(self.set_value(new_value, context, state)?)
            }
        }
    }
}
