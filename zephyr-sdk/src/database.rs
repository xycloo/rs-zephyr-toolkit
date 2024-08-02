use crate::{
    env::EnvClient,
    external::{env_push_stack, read_as_id, read_raw, update_raw, write_raw},
    symbol, to_fixed, SdkError,
};
use rs_zephyr_common::ZephyrVal;
use serde::{Deserialize, Serialize};
use soroban_sdk::xdr::{Limits, WriteXdr};

#[derive(Clone, Deserialize, Serialize)]
pub struct TypeWrap(pub Vec<u8>);

impl TypeWrap {
    pub fn to_i128(&self) -> i128 {
        let bytes = to_fixed::<u8, 16>(self.0.clone());
        i128::from_be_bytes(bytes)
    }

    pub fn to_u64(&self) -> u64 {
        let bytes = to_fixed::<u8, 8>(self.0.clone());
        u64::from_be_bytes(bytes)
    }
}

/// Object returned by database reads.
/// It's a wrapper for table rows.
#[derive(Clone, Deserialize, Serialize)]
pub struct TableRows {
    /// Rows within the table
    pub rows: Vec<TableRow>,
}

/// Condition clauses that can be applied when reading the
/// database.
pub enum Condition {
    /// A given column is equal to a certain object.
    ColumnEqualTo(String, Vec<u8>),
}

/// Wraps a single row.
#[derive(Clone, Deserialize, Serialize)]
pub struct TableRow {
    /// Vector of wrapped columns.
    pub row: Vec<TypeWrap>,
}

mod unsafe_helpers {
    use crate::external::env_push_stack;

    pub(crate) unsafe fn push_head(table_name: i64, columns: Vec<i64>) {
        env_push_stack(table_name as i64);
        env_push_stack(columns.len() as i64);

        for col in columns {
            env_push_stack(col)
        }
    }

    pub(crate) unsafe fn push_data_segments(segments: Vec<(i64, i64)>) {
        env_push_stack(segments.len() as i64);

        for segment in segments {
            env_push_stack(segment.0);
            env_push_stack(segment.1);
        }
    }
}

#[derive(Clone, Default)]
pub struct Database {}

impl Database {
    pub fn read_table(
        table_name: &str,
        columns: &[&str],
        external_id: Option<i64>,
        conditions: Option<&[Condition]>,
    ) -> Result<TableRows, SdkError> {
        let table_name = symbol::Symbol::try_from_bytes(table_name.as_bytes()).unwrap();
        let cols = columns
            .into_iter()
            .map(|col| symbol::Symbol::try_from_bytes(col.as_bytes()).unwrap().0 as i64)
            .collect::<Vec<i64>>();

        unsafe { unsafe_helpers::push_head(table_name.0 as i64, cols) }

        if let Some(conditions) = conditions {
            unsafe {
                env_push_stack(conditions.len() as i64);

                let mut args = Vec::new();
                for cond in conditions {
                    let (colname, operator, value) = match cond {
                        Condition::ColumnEqualTo(colname, value) => (colname, 0, value),
                    };

                    env_push_stack(
                        symbol::Symbol::try_from_bytes(colname.as_bytes())
                            .unwrap()
                            .0 as i64,
                    );
                    env_push_stack(operator as i64);

                    args.push((value.as_ptr() as i64, value.len() as i64))
                }

                env_push_stack(args.len() as i64);

                for segment in args {
                    env_push_stack(segment.0);
                    env_push_stack(segment.1);
                }
            }
        };

        let (status, offset, size) = if let Some(external) = external_id {
            unsafe { read_as_id(external) }
        } else {
            unsafe { read_raw() }
        };
        SdkError::express_from_status(status)?;

        let table = {
            let memory: *const u8 = offset as *const u8;

            let slice = unsafe { core::slice::from_raw_parts(memory, size as usize) };

            if let Ok(table) = bincode::deserialize::<TableRows>(slice) {
                table
            } else {
                return Err(SdkError::Conversion);
            }
        };

        Ok(table)
    }

    pub fn write_table(
        table_name: &str,
        columns: &[&str],
        segments: &[&[u8]],
    ) -> Result<(), SdkError> {
        let table_name = symbol::Symbol::try_from_bytes(table_name.as_bytes()).unwrap();
        let cols = columns
            .into_iter()
            .map(|col| symbol::Symbol::try_from_bytes(col.as_bytes()).unwrap().0 as i64)
            .collect::<Vec<i64>>();

        let segments = segments
            .into_iter()
            .map(|segment| (segment.as_ptr() as i64, segment.len() as i64))
            .collect::<Vec<(i64, i64)>>();

        unsafe {
            unsafe_helpers::push_head(table_name.0 as i64, cols);
            unsafe_helpers::push_data_segments(segments);
        }

        let status = unsafe { write_raw() };
        SdkError::express_from_status(status)
    }

    pub fn update_table(
        table_name: &str,
        columns: &[&str],
        segments: &[&[u8]],
        conditions: &[Condition],
    ) -> Result<(), SdkError> {
        let table_name = symbol::Symbol::try_from_bytes(table_name.as_bytes()).unwrap();
        let cols = columns
            .into_iter()
            .map(|col| symbol::Symbol::try_from_bytes(col.as_bytes()).unwrap().0 as i64)
            .collect::<Vec<i64>>();

        let segments = segments
            .into_iter()
            .map(|segment| (segment.as_ptr() as i64, segment.len() as i64))
            .collect::<Vec<(i64, i64)>>();

        unsafe {
            unsafe_helpers::push_head(table_name.0 as i64, cols);
            unsafe_helpers::push_data_segments(segments);

            env_push_stack(conditions.len() as i64);

            let mut args = Vec::new();
            for cond in conditions {
                let (colname, operator, value) = match cond {
                    Condition::ColumnEqualTo(colname, value) => (colname, 0, value),
                };

                env_push_stack(
                    symbol::Symbol::try_from_bytes(colname.as_bytes())
                        .unwrap()
                        .0 as i64,
                );
                env_push_stack(operator as i64);

                args.push((value.as_ptr() as i64, value.len() as i64))
            }

            env_push_stack(args.len() as i64);

            for segment in args {
                env_push_stack(segment.0);
                env_push_stack(segment.1);
            }
        }

        let status = unsafe { update_raw() };
        SdkError::express_from_status(status)
    }
}

#[derive(PartialEq)]
pub(crate) enum Action {
    Read,
    Update,
}

/// Simple wrapper for building conditions.
pub struct TableQueryWrapper {
    conditions: Vec<Condition>,
    action: Action,
}

impl TableQueryWrapper {
    /// Creates a new table update object.
    pub(crate) fn new(action: Action) -> Self {
        Self {
            conditions: vec![],
            action,
        }
    }

    /// Adds a new condition in the update according to which a given column
    /// should be equal to an XDR object.
    pub fn column_equal_to_xdr(&mut self, column: impl ToString, xdr: &impl WriteXdr) -> &mut Self {
        let bytes = xdr.to_xdr(Limits::none()).unwrap();
        let condition = Condition::ColumnEqualTo(column.to_string(), bytes);
        self.conditions.push(condition);

        self
    }

    /// Adds a new condition in the update according to which a given column
    /// should be equal to the matching bytes array.
    ///
    /// This filter should be used when dealing with non-XDR types. Serialization
    /// must be carried by the implementor.
    pub fn column_equal_to_bytes(&mut self, column: impl ToString, bytes: &[u8]) -> &mut Self {
        let condition = Condition::ColumnEqualTo(column.to_string(), bytes.to_vec());
        self.conditions.push(condition);

        self
    }

    /// Adds a new condition in the update according to which a given column
    /// should be equal to the matching object.
    /// 
    /// Under the hood, the object is converted to a ZephyrVal and is later
    /// serialized. 
    pub fn column_equal_to<T: Serialize + TryInto<ZephyrVal>>(
        &mut self,
        column: impl ToString,
        argument: T,
    ) -> &mut Self {
        let argument = bincode::serialize(
            &TryInto::<ZephyrVal>::try_into(argument)
                .map_err(|_| ())
                .unwrap(),
        )
        .unwrap();
        let condition = Condition::ColumnEqualTo(column.to_string(), argument);
        self.conditions.push(condition);

        self
    }

    /// Executes the update.
    /// Note: should only be used when updating a table.
    pub fn execute(&mut self, interact: &impl DatabaseInteract) -> Result<(), SdkError> {
        if self.action != Action::Update {
            return Err(SdkError::ReadOnUpdateAction);
        }

        Ok(interact.update(&EnvClient::empty(), &self.conditions))
    }

    /// Executes the query and returns the results.
    pub fn read<T: DatabaseInteract>(&self) -> Result<Vec<T>, SdkError> {
        let env = EnvClient::empty();

        if self.action != Action::Read {
            return Err(SdkError::UpdateOnReadAction);
        }

        Ok(T::read_to_rows(&env, Some(&self.conditions)))
    }
}

/// Trait that DatabaseDerive structures implement
pub trait DatabaseInteract {
    /// Reads from the database into a vector of `Self`.
    fn read_to_rows(env: &EnvClient, conditions: Option<&[Condition]>) -> Vec<Self>
    where
        Self: Sized;

    /// Inserts a row `Self` into the database table.
    fn put(&self, env: &EnvClient);

    /// Updates an existing row with `Self` into the database table
    /// using the provided conditions as update filter.
    fn update(&self, env: &EnvClient, conditions: &[Condition]);
}
