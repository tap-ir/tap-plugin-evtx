//! evtx plugin take a VFile attribute return from a node and  add the result of an evtx function to the attribute of this node

use std::fmt::Debug;

use tap::config_schema;
use tap::plugin;
use tap::plugin::{PluginInfo, PluginInstance, PluginConfig, PluginArgument, PluginResult, PluginEnvironment};
use tap::tree::{TreeNodeId, TreeNodeIdSchema};
use tap::attribute::{Attributes};
use tap::value::Value;
use tap::error::RustructError;
use tap::node::Node;

use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};
use schemars::{JsonSchema};
use evtx::EvtxParser;

plugin!("evtx", "Windows", "Parse evtx file", EvtxPlugin, Arguments);

#[derive(Debug, Serialize, Deserialize,JsonSchema)]
pub struct Arguments
{
  #[schemars(with = "TreeNodeIdSchema")] 
  file : TreeNodeId,
}

#[derive(Debug, Serialize, Deserialize,Default)]
pub struct Results
{
}

#[derive(Default)]
pub struct EvtxPlugin
{
}

impl EvtxPlugin
{
  fn run(&mut self, args : Arguments, env : PluginEnvironment) -> Result<Results>
  {
    let file_node = env.tree.get_node_from_id(args.file).ok_or(RustructError::ArgumentNotFound("file"))?;
    file_node.value().add_attribute(self.name(), None, None); 
    let data = file_node.value().get_value("data").ok_or(RustructError::ValueNotFound("data"))?;
    let data_builder = data.try_as_vfile_builder().ok_or(RustructError::ValueTypeMismatch)?;
    let mut file = data_builder.open()?;
    
    let mut evtx_parser = match EvtxParser::from_read_seek(&mut file)
    {
       Ok(evtx) => evtx,
       Err(err) => return Err(anyhow!(err.to_string())),
    };
      
    for record in evtx_parser.records_json_value().flatten()
    {
      let node_record = Node::new(record.event_record_id.to_string());

      let attribute = json_value_to_core_value(record.data);
      attribute.as_attributes().add_attribute("time", record.timestamp, None);
      node_record.value().add_attribute("evtx", attribute, None);
      env.tree.add_child(args.file, node_record)?;
    }

    Ok(Results{})
  }
}

fn json_value_to_core_value(value : serde_json::Value) -> Value
{  
  match value
  { 
    serde_json::Value::Number(number) => 
    {
      match number.as_u64()
      {
        Some(number) =>  Value::from(number),
        None => Value::from(None),
      }
    }
    serde_json::Value::String(string) => Value::from(string),
    serde_json::Value::Object(map) => 
    {
      let mut attributes = Attributes::new();
      for j_attr in map
      {
        attributes.add_attribute(j_attr.0.to_lowercase(), json_value_to_core_value(j_attr.1), None);
      }
      Value::from(attributes)
    },
    _ => Value::from(None),
  }
}
