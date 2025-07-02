use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CompactSize {
    pub value: u64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BitcoinError {
    InsufficientBytes,
    InvalidFormat,
}

impl CompactSize {
    pub fn new(value: u64) -> Self {
        CompactSize { value }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        if self.value < 253 {
            bytes.push(self.value as u8);
        } else if self.value <= 0xFFFF {
            bytes.push(253);
            bytes.extend_from_slice(&self.value.to_le_bytes()[..2]);
        } else if self.value <= 0xFFFFFFFF {
            bytes.push(254);
            bytes.extend_from_slice(&self.value.to_le_bytes()[..4]);
        } else {
            bytes.push(255);
            bytes.extend_from_slice(&self.value.to_le_bytes());
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.is_empty() {
            return Err(BitcoinError::InsufficientBytes);
        }

        let prefix = bytes[0];
        match prefix {
            0..=252 => Ok((
                CompactSize {
                    value: prefix as u64,
                },
                1,
            )),
            253 => {
                if bytes.len() < 3 {
                    return Err(BitcoinError::InsufficientBytes);
                }
                let value = u16::from_le_bytes([bytes[1], bytes[2]]) as u64;
                Ok((CompactSize { value }, 3))
            }
            254 => {
                if bytes.len() < 5 {
                    return Err(BitcoinError::InsufficientBytes);
                }
                let value = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as u64;
                Ok((CompactSize { value }, 5))
            }
            255 => {
                if bytes.len() < 9 {
                    return Err(BitcoinError::InsufficientBytes);
                }
                let value = u64::from_le_bytes([
                    bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
                ]);
                Ok((CompactSize { value }, 9))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Txid(pub [u8; 32]);

impl Serialize for Txid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_string = hex::encode(self.0);
        serializer.serialize_str(&hex_string)
    }
}

impl<'de> Deserialize<'de> for Txid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_string = String::deserialize(deserializer)?;
        let decoded = hex::decode(&hex_string).map_err(serde::de::Error::custom)?;
        if decoded.len() != 32 {
            return Err(serde::de::Error::custom("Invalid Txid length"));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&decoded);
        Ok(Txid(array))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct OutPoint {
    pub txid: Txid,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: [u8; 32], vout: u32) -> Self {
        OutPoint {
            txid: Txid(txid),
            vout,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.txid.0);
        bytes.extend_from_slice(&self.vout.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.len() < 36 {
            return Err(BitcoinError::InsufficientBytes);
        }

        let mut txid = [0u8; 32];
        txid.copy_from_slice(&bytes[0..32]);
        let vout = u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);

        Ok((
            OutPoint {
                txid: Txid(txid),
                vout,
            },
            36,
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Script {
    pub bytes: Vec<u8>,
}

impl Script {
    pub fn new(bytes: Vec<u8>) -> Self {
        Script { bytes }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let compact_size = CompactSize::new(self.bytes.len() as u64);
        bytes.extend_from_slice(&compact_size.to_bytes());
        bytes.extend_from_slice(&self.bytes);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        let (compact_size, prefix_size) = CompactSize::from_bytes(bytes)?;
        let script_length = compact_size.value as usize;

        if bytes.len() < prefix_size + script_length {
            return Err(BitcoinError::InsufficientBytes);
        }

        let script_bytes = bytes[prefix_size..prefix_size + script_length].to_vec();
        Ok((
            Script {
                bytes: script_bytes,
            },
            prefix_size + script_length,
        ))
    }
}

impl Deref for Script {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    pub previous_output: OutPoint,
    pub script_sig: Script,
    pub sequence: u32,
}

impl TransactionInput {
    pub fn new(previous_output: OutPoint, script_sig: Script, sequence: u32) -> Self {
        TransactionInput {
            previous_output,
            script_sig,
            sequence,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.previous_output.to_bytes());
        bytes.extend_from_slice(&self.script_sig.to_bytes());
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        let (previous_output, outpoint_size) = OutPoint::from_bytes(bytes)?;
        let (script_sig, script_size) = Script::from_bytes(&bytes[outpoint_size..])?;

        let sequence_start = outpoint_size + script_size;
        if bytes.len() < sequence_start + 4 {
            return Err(BitcoinError::InsufficientBytes);
        }

        let sequence = u32::from_le_bytes([
            bytes[sequence_start],
            bytes[sequence_start + 1],
            bytes[sequence_start + 2],
            bytes[sequence_start + 3],
        ]);

        Ok((
            TransactionInput {
                previous_output,
                script_sig,
                sequence,
            },
            sequence_start + 4,
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    pub version: u32,
    pub inputs: Vec<TransactionInput>,
    pub lock_time: u32,
}

impl BitcoinTransaction {
    pub fn new(version: u32, inputs: Vec<TransactionInput>, lock_time: u32) -> Self {
        BitcoinTransaction {
            version,
            inputs,
            lock_time,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.to_le_bytes());

        let input_count = CompactSize::new(self.inputs.len() as u64);
        bytes.extend_from_slice(&input_count.to_bytes());

        for input in &self.inputs {
            bytes.extend_from_slice(&input.to_bytes());
        }

        bytes.extend_from_slice(&self.lock_time.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.len() < 4 {
            return Err(BitcoinError::InsufficientBytes);
        }

        let version = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let (input_count, input_count_size) = CompactSize::from_bytes(&bytes[4..])?;
        let mut inputs = Vec::new();
        let mut offset = 4 + input_count_size;

        for _ in 0..input_count.value {
            let (input, input_size) = TransactionInput::from_bytes(&bytes[offset..])?;
            inputs.push(input);
            offset += input_size;
        }

        if bytes.len() < offset + 4 {
            return Err(BitcoinError::InsufficientBytes);
        }

        let lock_time = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);

        Ok((
            BitcoinTransaction {
                version,
                inputs,
                lock_time,
            },
            offset + 4,
        ))
    }
}

impl fmt::Display for BitcoinTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BitcoinTransaction:")?;
        writeln!(f, "  Version: {}", self.version)?;
        writeln!(f, "  Lock Time: {}", self.lock_time)?;
        writeln!(f, "  Inputs:")?;
        for (i, input) in self.inputs.iter().enumerate() {
            writeln!(f, "    Input {}:", i + 1)?;
            writeln!(
                f,
                "      Previous Output: Txid: {}, Vout: {}",
                hex::encode(input.previous_output.txid.0),
                input.previous_output.vout
            )?;
            writeln!(
                f,
                "      ScriptSig ({} bytes): {:?}",
                input.script_sig.len(),
                input.script_sig
            )?;
            writeln!(f, "      Sequence: {}", input.sequence)?;
        }
        Ok(())
    }
}
