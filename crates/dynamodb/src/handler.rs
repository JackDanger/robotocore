//! DynamoDB operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{AttributeDefinition, DynamoDbState, Item, KeySchema, Table};
use crate::protocol::{AwsRequest, AwsResponse};

/// The DynamoDB service handler.
pub struct DynamoDbHandler {
    state: RwLock<HashMap<(u64, String), DynamoDbState>>,
}

impl DynamoDbHandler {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(HashMap::new()),
        }
    }

    fn get_state(&self, account: u64, region: &str) -> DynamoDbState {
        let mut states = self.state.write();
        states
            .entry((account, region.to_string()))
            .or_insert_with(DynamoDbState::new)
            .clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let operation = req.operation.as_str();
        match operation {
            "ListTables" => self.list_tables(&req),
            "CreateTable" => self.create_table(&req),
            "DeleteTable" => self.delete_table(&req),
            "DescribeTable" => self.describe_table(&req),
            "PutItem" => self.put_item(&req),
            "GetItem" => self.get_item(&req),
            "DeleteItem" => self.delete_item(&req),
            "Query" => self.query(&req),
            "Scan" => self.scan(&req),
            "UpdateItem" => self.update_item(&req),
            "BatchGetItem" => self.batch_get_item(&req),
            "BatchWriteItem" => self.batch_write_item(&req),
            "UpdateTable" => self.update_table(&req),
            "TagResource" => self.tag_resource(&req),
            "UntagResource" => self.untag_resource(&req),
            "ListTagsOfResource" => self.list_tags_of_resource(&req),
            "DescribeLimits" => self.describe_limits(&req),
            "DescribeTimeToLive" => self.describe_ttl(&req),
            "UpdateTimeToLive" => self.update_ttl(&req),
            "ConditionCheck" => self.condition_check(&req),
            "TransactWriteItems" => self.json_stub(&req, "ClientRequestToken"),
            "TransactGetItems" => self.json_stub_list(&req, "Responses"),
            "BatchExecuteStatement" => self.json_stub_list(&req, "Responses"),
            "ExecuteStatement" => self.json_stub_list(&req, "Items"),
            "ExecuteTransaction" => self.json_stub(&req, "ConsumedCapacity"),
            "DescribeContinuousBackups" => self.json_stub(&req, "ContinuousBackupsDescription"),
            "UpdateContinuousBackups" => self.json_stub(&req, "ContinuousBackupsDescription"),
            "CreateBackup" => self.json_stub(&req, "BackupDescription"),
            "DescribeBackup" => self.json_stub(&req, "BackupDescription"),
            "ListBackups" => self.json_stub_list(&req, "BackupSummaries"),
            "DeleteBackup" => self.json_stub(&req, "{}"),
            "RestoreTableFromBackup" => self.json_stub(&req, "TableDescription"),
            "RestoreTableToPointInTime" => self.json_stub(&req, "TableDescription"),
            "GetResourcePolicy" => self.json_stub(&req, "ResourcePolicy"),
            "PutResourcePolicy" => self.json_stub(&req, "{}"),
            "DeleteResourcePolicy" => self.json_stub(&req, "{}"),
            other => AwsResponse::error(
                400,
                "ValidationException",
                &format!("The operation {} is not implemented", other),
            ),
        }
    }

    fn list_tables(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let tables = state.list_tables();
        AwsResponse::json(200, json!({ "TableNames": tables }))
    }

    fn create_table(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req
            .params
            .get("TableName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if table_name.is_empty() {
            return AwsResponse::error(400, "ValidationException", "TableName is required");
        }

        let state = self.get_state(req.account, &req.region);
        if state.get_table(&table_name).is_some() {
            return AwsResponse::error(
                400,
                "TableAlreadyExistsException",
                &format!("Table already exists: {}", table_name),
            );
        }

        let key_schema: Vec<KeySchema> = req
            .params
            .get("KeySchema")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|k| {
                        Some(KeySchema {
                            attribute_name: k.get("AttributeName")?.as_str()?.to_string(),
                            key_type: k.get("KeyType")?.as_str()?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let attribute_definitions: Vec<AttributeDefinition> = req
            .params
            .get("AttributeDefinitions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        Some(AttributeDefinition {
                            attribute_name: a.get("AttributeName")?.as_str()?.to_string(),
                            attribute_type: a.get("AttributeType")?.as_str()?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let billing_mode = req
            .params
            .get("BillingMode")
            .and_then(|v| v.as_str())
            .unwrap_or("PAY_PER_REQUEST")
            .to_string();

        let table = Arc::new(Table::new(
            table_name.clone(),
            req.account,
            req.region.clone(),
            key_schema.clone(),
            attribute_definitions.clone(),
            billing_mode,
        ));
        state.put_table(table.clone());

        let table_arn = format!("arn:aws:dynamodb:{}:{}:table/{}", req.region, req.account, table_name);
        let key_schema_json: Vec<Value> = table
            .key_schema
            .iter()
            .map(|k| json!({"AttributeName": k.attribute_name, "KeyType": k.key_type}))
            .collect();
        let attr_defs_json: Vec<Value> = table
            .attribute_definitions
            .iter()
            .map(|a| json!({"AttributeName": a.attribute_name, "AttributeType": a.attribute_type}))
            .collect();

        AwsResponse::json(
            200,
            json!({
                "TableDescription": {
                    "TableName": table_name,
                    "TableStatus": "ACTIVE",
                    "TableId": table.table_id.clone(),
                    "KeySchema": key_schema_json,
                    "AttributeDefinitions": attr_defs_json,
                    "TableArn": table_arn,
                    "CreationRequestTime": table.created_at as f64,
                    "CreationDateTime": table.created_at as f64,
                    "ProvisionedThroughput": {
                        "ReadCapacityUnits": 0.0,
                        "WriteCapacityUnits": 0.0
                    },
                    "LocalSecondaryIndexes": [],
                    "GlobalSecondaryIndexes": [],
                    "DeletionProtectionEnabled": false,
                    "SseSpecification": { "SseType": "DISABLED" },
                    "TableId": table.table_id.clone(),
                    "BillingModeSummary": { "BillingMode": table.billing_mode }
                }
            }),
        )
    }

    fn delete_table(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req
            .params
            .get("TableName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let state = self.get_state(req.account, &req.region);
        match state.delete_table(&table_name) {
            Some(_) => AwsResponse::json(200, json!({
                "TableDescription": { "TableName": table_name, "TableStatus": "DELETING" }
            })),
            None => AwsResponse::error(
                400,
                "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name),
            ),
        }
    }

    fn describe_table(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req
            .params
            .get("TableName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let state = self.get_state(req.account, &req.region);
        match state.get_table(&table_name) {
            Some(table) => {
                let table_arn = format!("arn:aws:dynamodb:{}:{}:table/{}", req.region, req.account, table.name);
                let key_schema_json: Vec<Value> = table
                    .key_schema
                    .iter()
                    .map(|k| json!({"AttributeName": k.attribute_name, "KeyType": k.key_type}))
                    .collect();
                let attr_defs_json: Vec<Value> = table
                    .attribute_definitions
                    .iter()
                    .map(|a| json!({"AttributeName": a.attribute_name, "AttributeType": a.attribute_type}))
                    .collect();
                let item_count = table.items.read().len() as i64;

                AwsResponse::json(200, json!({
                    "Table": {
                        "TableName": table.name,
                        "TableStatus": *table.status.read(),
                        "KeySchema": key_schema_json,
                        "AttributeDefinitions": attr_defs_json,
                        "TableArn": table_arn,
                        "ItemCount": item_count,
                        "TableSizeBytes": 0,
                        "CreationRequestTime": table.created_at as f64,
                        "BillingMode": table.billing_mode,
                        "ProvisionedThroughput": {
                            "ReadCapacityUnits": 0.0,
                            "WriteCapacityUnits": 0.0
                        },
                        "LocalSecondaryIndexes": [],
                        "GlobalSecondaryIndexes": [],
                        "DeletionProtectionEnabled": false,
                        "SseSpecification": { "SseType": "DISABLED" },
                        "TableId": table.table_id.clone(),
                        "BillingModeSummary": { "BillingMode": table.billing_mode }
                    }
                }))
            }
            None => AwsResponse::error(
                400,
                "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name),
            ),
        }
    }

    fn put_item(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let table = match state.get_table(&table_name) {
            Some(t) => t,
            None => return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name)),
        };

        let item_json = req.params.get("Item").cloned().unwrap_or(Value::Null);
        let attributes: HashMap<String, Value> = item_json
            .as_object()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let item = Item::new(attributes);
        let key = match table.compute_key(&item) {
            Some(k) => k,
            None => return AwsResponse::error(400, "ValidationException", "Missing primary key in item"),
        };

        table.items.write().insert(key, Arc::new(item));
        AwsResponse::json(200, json!({}))
    }

    fn get_item(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let table = match state.get_table(&table_name) {
            Some(t) => t,
            None => return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name)),
        };

        let key_json = req.params.get("Key").cloned().unwrap_or(Value::Null);
        let key_attrs: HashMap<String, Value> = key_json
            .as_object()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let items = table.items.read();
        for (_key_str, item) in items.iter() {
            if key_attrs.iter().all(|(name, val)| item.attributes.get(name) == Some(val)) {
                let item_json = serde_json::to_value(&item.attributes).unwrap_or(Value::Null);
                return AwsResponse::json(200, json!({ "Item": item_json }));
            }
        }
        AwsResponse::json(200, json!({}))
    }

    fn delete_item(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let table = match state.get_table(&table_name) {
            Some(t) => t,
            None => return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name)),
        };

        let key_json = req.params.get("Key").cloned().unwrap_or(Value::Null);
        let key_attrs: HashMap<String, Value> = key_json
            .as_object()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let key_to_delete = {
            let items = table.items.read();
            items.iter().find(|(_, item)| {
                key_attrs.iter().all(|(name, val)| item.attributes.get(name) == Some(val))
            }).map(|(k, _)| k.clone())
        };

        if let Some(k) = key_to_delete {
            table.items.write().remove(&k);
        }
        AwsResponse::json(200, json!({}))
    }

    fn query(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let table = match state.get_table(&table_name) {
            Some(t) => t,
            None => return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name)),
        };

        let items = table.items.read();
        let mut result_items: Vec<Value> = Vec::new();
        for (_key, item) in items.iter() {
            let item_json = serde_json::to_value(&item.attributes).unwrap_or(Value::Null);
            result_items.push(item_json);
        }

        let limit = req.params.get("Limit").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(1000);
        let items_slice = &result_items[..result_items.len().min(limit)];

        AwsResponse::json(200, json!({
            "Items": items_slice,
            "Count": items_slice.len(),
            "ScannedCount": items_slice.len()
        }))
    }

    fn scan(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let table = match state.get_table(&table_name) {
            Some(t) => t,
            None => return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name)),
        };

        let items = table.items.read();
        let mut result_items: Vec<Value> = Vec::new();
        for (_key, item) in items.iter() {
            let item_json = serde_json::to_value(&item.attributes).unwrap_or(Value::Null);
            result_items.push(item_json);
        }
        drop(items);

        let limit = req.params.get("Limit").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(1000);
        let items_slice = &result_items[..result_items.len().min(limit)];

        AwsResponse::json(200, json!({
            "Items": items_slice,
            "Count": items_slice.len(),
            "ScannedCount": items_slice.len(),
            "ConsumedCapacity": { "TableName": table_name, "CapacityUnits": 1.0 }
        }))
    }

    fn update_item(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let table = match state.get_table(&table_name) {
            Some(t) => t,
            None => return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name)),
        };

        let key_json = req.params.get("Key").cloned().unwrap_or(Value::Null);
        let update_expr = req.params.get("UpdateExpression").and_then(|v| v.as_str()).unwrap_or("");
        let expression_values = req.params.get("ExpressionAttributeValues").cloned().unwrap_or(Value::Null);
        let return_values = req.params.get("ReturnValues").and_then(|v| v.as_str()).unwrap_or("NONE");

        // Find the item
        let key_attrs: HashMap<String, Value> = key_json
            .as_object()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let items = table.items.read();
        let item_key = items.iter().find(|(_, item)| {
            key_attrs.iter().all(|(name, val)| item.attributes.get(name) == Some(val))
        });

        let (item_key, _item) = match item_key {
            Some((k, item)) => (k.clone(), item.clone()),
            None => {
                if !update_expr.contains("ADD") {
                    // For non-ADD updates, the item must exist
                    return AwsResponse::json(200, json!({ "ConsumedCapacity": null }));
                }
                // For ADD, create the item if it doesn't exist
                drop(items);
                let mut new_attrs: HashMap<String, Value> = key_attrs.clone();
                // Apply ADD expression to create new item
                self.apply_add_expr(&mut new_attrs, update_expr, &expression_values);
                let new_item = Item::new(new_attrs.clone());
                if let Some(key) = table.compute_key(&new_item) {
                    table.items.write().insert(key, Arc::new(new_item));
                }
                if return_values == "ALL_NEW" {
                    let item_json = serde_json::to_value(&new_attrs).unwrap_or(Value::Null);
                    return AwsResponse::json(200, json!({ "Item": item_json }));
                }
                return AwsResponse::json(200, json!({}));
            }
        };

        drop(items);

        // Get the item and apply the update
        let mut items = table.items.write();
        let item = match items.get(&item_key) {
            Some(item) => item.clone(),
            None => return AwsResponse::json(200, json!({})),
        };

        let mut attrs: HashMap<String, Value> = item.attributes.clone();

        // Parse and apply the update expression
        self.apply_update_expr(&mut attrs, update_expr, &expression_values);

        // Update the item
        let updated_item = Item::new(attrs.clone());
        items.insert(item_key.clone(), Arc::new(updated_item));
        drop(items);

        // Build response
        let mut resp = json!({});
        if return_values == "ALL_NEW" {
            resp["Item"] = serde_json::to_value(&attrs).unwrap_or(Value::Null);
        } else if return_values == "ALL_OLD" {
            resp["Item"] = serde_json::to_value(&item.attributes).unwrap_or(Value::Null);
        }
        AwsResponse::json(200, resp)
    }

    fn apply_update_expr(&self, attrs: &mut HashMap<String, Value>, expr: &str, values: &Value) {
        let expr = expr.trim();

        // Handle SET clause
        if let Some(set_pos) = expr.find("SET") {
            let rest = expr[set_pos + 3..].trim();
            // Parse SET attr = value pairs
            let pairs: Vec<&str> = rest.split(",").map(|s| s.trim()).collect();
            for pair in pairs {
                if let Some(eq_pos) = pair.find('=') {
                    let attr = pair[..eq_pos].trim();
                    let val_expr = pair[eq_pos+1..].trim();

                    // Resolve the value
                    let val = if val_expr.starts_with(':') {
                        // Expression attribute value
                        values.get(val_expr).cloned().unwrap_or(Value::Null)
                    } else if val_expr.contains(" + ") || val_expr.contains(" - ") {
                        // Arithmetic: attr + :val or attr - :val
                        let parts: Vec<&str> = val_expr.split_whitespace().collect();
                        if parts.len() == 3 {
                            let a_name = parts[0];
                            let op = parts[1];
                            let val_ref = parts[2];
                            let current = attrs.get(a_name).cloned().unwrap_or(json!(0));
                            let add_val = values.get(val_ref).cloned().unwrap_or(json!(0));
                            let cur_num: f64 = self.dyn_to_f64(&current);
                            let add_num: f64 = self.dyn_to_f64(&add_val);
                            let result = if op == "+" { cur_num + add_num } else { cur_num - add_num };
                            json!({ "N": result.to_string() })
                        } else {
                            json!({ "S": val_expr })
                        }
                    } else if val_expr.starts_with("if_not_exists") {
                        let inner = val_expr[val_expr.find('(').unwrap() + 1..val_expr.rfind(')').unwrap()].trim();
                        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                        if parts.len() == 2 {
                            let a_name = parts[0];
                            let val_ref = parts[1];
                            if attrs.contains_key(a_name) {
                                attrs.get(a_name).cloned().unwrap_or(Value::Null)
                            } else {
                                values.get(val_ref).cloned().unwrap_or(Value::Null)
                            }
                        } else {
                            json!({ "S": val_expr })
                        }
                    } else {
                        json!({ "S": val_expr })
                    };

                    attrs.insert(attr.to_string(), val);
                }
            }
        }

        // Handle ADD clause
        if let Some(add_pos) = expr.find("ADD") {
            let rest = expr[add_pos + 3..].trim();
            self.apply_add_expr(attrs, rest, values);
        }

        // Handle REMOVE clause
        if let Some(remove_pos) = expr.find("REMOVE") {
            let rest = expr[remove_pos + 6..].trim();
            let names: Vec<&str> = rest.split(",").map(|s| s.trim()).collect();
            for name in names {
                attrs.remove(name);
            }
        }
    }

    fn apply_add_expr(&self, attrs: &mut HashMap<String, Value>, expr: &str, values: &Value) {
        // ADD attr :val, attr2 :val2
        let parts: Vec<&str> = expr.split(",").map(|s| s.trim()).collect();
        for part in parts {
            let words: Vec<&str> = part.split_whitespace().collect();
            if words.len() == 2 {
                let attr = words[0];
                let val_ref = words[1];
                let add_val = values.get(val_ref).cloned().unwrap_or(json!(0));
                let current = attrs.get(attr).cloned().unwrap_or(json!({ "N": "0" }));
                let cur_num: f64 = self.dyn_to_f64(&current);
                let add_num: f64 = self.dyn_to_f64(&add_val);
                let result = cur_num + add_num;
                // Format as integer if it's a whole number
                if result == result.floor() {
                    attrs.insert(attr.to_string(), json!({ "N": format!("{}", result as i64) }));
                } else {
                    attrs.insert(attr.to_string(), json!({ "N": result.to_string() }));
                }
            }
        }
    }

    fn dyn_to_f64(&self, v: &Value) -> f64 {
        match v {
            Value::String(s) => s.parse().unwrap_or(0.0),
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            Value::Object(obj) => {
                // DynamoDB type: {"N": "1"} or {"S": "abc"}
                obj.get("N").and_then(|n| n.as_str()).and_then(|s| s.parse().ok())
                    .or_else(|| obj.get("S").and_then(|s| s.as_str()).and_then(|s| s.parse().ok()))
                    .unwrap_or(0.0)
            }
            _ => 0.0,
        }
    }

    fn batch_get_item(&self, req: &AwsRequest) -> AwsResponse {
        let request_items = req.params.get("RequestItems").cloned().unwrap_or(Value::Null);
        let mut responses: HashMap<String, Vec<Value>> = HashMap::new();

        if let Some(obj) = request_items.as_object() {
            for (table_name, keys_json) in obj {
                let state = self.get_state(req.account, &req.region);
                let table = match state.get_table(table_name) {
                    Some(t) => t,
                    None => continue,
                };

                let keys = keys_json.get("Keys").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let items = table.items.read();
                let mut found: Vec<Value> = Vec::new();
                for key_json in &keys {
                    let key_attrs: HashMap<String, Value> = key_json
                        .as_object()
                        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default();
                    for (_key_str, item) in items.iter() {
                        if key_attrs.iter().all(|(name, val)| item.attributes.get(name) == Some(val)) {
                            let item_json = serde_json::to_value(&item.attributes).unwrap_or(Value::Null);
                            found.push(item_json);
                            break;
                        }
                    }
                }
                responses.insert(table_name.clone(), found);
            }
        }

        AwsResponse::json(200, json!({ "Responses": responses, "UnprocessedKeys": {} }))
    }

    fn batch_write_item(&self, req: &AwsRequest) -> AwsResponse {
        let request_items = req.params.get("RequestItems").cloned().unwrap_or(Value::Null);

        if let Some(obj) = request_items.as_object() {
            for (table_name, writes_json) in obj {
                let state = self.get_state(req.account, &req.region);
                let table = match state.get_table(table_name) {
                    Some(t) => t,
                    None => continue,
                };

                let writes = writes_json.as_array().cloned().unwrap_or_default();
                for write in &writes {
                    if let Some(item_json) = write.get("PutRequest").and_then(|v| v.get("Item")) {
                        let attributes: HashMap<String, Value> = item_json
                            .as_object()
                            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            .unwrap_or_default();
                        let item = Item::new(attributes);
                        if let Some(key) = table.compute_key(&item) {
                            table.items.write().insert(key, Arc::new(item));
                        }
                    }
                    if let Some(key_json) = write.get("DeleteRequest").and_then(|v| v.get("Key")) {
                        let key_attrs: HashMap<String, Value> = key_json
                            .as_object()
                            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            .unwrap_or_default();
                        let key_to_delete = {
                            let items = table.items.read();
                            items.iter().find(|(_, item)| {
                                key_attrs.iter().all(|(name, val)| item.attributes.get(name) == Some(val))
                            }).map(|(k, _)| k.clone())
                        };
                        if let Some(k) = key_to_delete {
                            table.items.write().remove(&k);
                        }
                    }
                }
            }
        }

        AwsResponse::json(200, json!({ "UnprocessedItems": {} }))
    }

    fn update_table(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        match state.get_table(&table_name) {
            Some(table) => AwsResponse::json(200, json!({
                "TableDescription": { "TableName": table_name, "TableStatus": *table.status.read() }
            })),
            None => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name)),
        }
    }

    fn tag_resource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn untag_resource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn list_tags_of_resource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({ "Tags": [] }))
    }

    fn describe_limits(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "MaxNumberOfTables": 100,
            "MaxGlobalSecondaryIndexesPerTable": 100
        }))
    }
    // ---- JSON stub helpers ----
    fn json_empty(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, serde_json::json!({}))
    }
    fn describe_ttl(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        if state.get_table(&table_name).is_some() {
            AwsResponse::json(200, json!({
                "TimeToLiveDescription": {
                    "TableName": table_name,
                    "TimeToLiveStatus": "DISABLED"
                }
            }))
        } else {
            AwsResponse::error(400, "ResourceNotFoundException", &format!("Table not found: {}", table_name))
        }
    }

    fn update_ttl(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "TimeToLiveSpecification": {
                "TableName": "unknown",
                "TimeToLive": { "Enabled": false, "AttributeName": "" }
            }
        }))
    }

    fn condition_check(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        if state.get_table(&table_name).is_none() {
            return AwsResponse::error(400, "ResourceNotFoundException", &format!("Table not found: {}", table_name));
        }
        // Simplified: always return true (no real condition evaluation)
        AwsResponse::json(200, json!({
            "ConditionalCheckFalse": false,
            "Item": {}
        }))
    }

    fn json_stub(&self, _req: &AwsRequest, field: &str) -> AwsResponse {
        let mut obj = serde_json::Map::new();
        obj.insert(field.to_string(), serde_json::json!({}));
        AwsResponse::json(200, serde_json::Value::Object(obj))
    }
    fn json_stub_list(&self, _req: &AwsRequest, field: &str) -> AwsResponse {
        let mut obj = serde_json::Map::new();
        obj.insert(field.to_string(), serde_json::json!([]));
        AwsResponse::json(200, serde_json::Value::Object(obj))
    }

}

impl Default for DynamoDbHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "dynamodb".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_create_and_list_tables() {
        let handler = DynamoDbHandler::new();
        let req = make_req("CreateTable", json!({
            "TableName": "test-table",
            "KeySchema": [{"AttributeName": "id", "KeyType": "HASH"}],
            "AttributeDefinitions": [{"AttributeName": "id", "AttributeType": "S"}],
            "BillingMode": "PAY_PER_REQUEST"
        }));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);

        let req = make_req("ListTables", json!({}));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("test-table"));
    }

    #[test]
    fn test_put_and_get_item() {
        let handler = DynamoDbHandler::new();
        handler.handle(make_req("CreateTable", json!({
            "TableName": "users",
            "KeySchema": [{"AttributeName": "id", "KeyType": "HASH"}],
            "AttributeDefinitions": [{"AttributeName": "id", "AttributeType": "S"}],
            "BillingMode": "PAY_PER_REQUEST"
        })));

        handler.handle(make_req("PutItem", json!({
            "TableName": "users",
            "Item": { "id": {"S": "u1"}, "name": {"S": "Alice"} }
        })));

        let req = make_req("GetItem", json!({
            "TableName": "users",
            "Key": { "id": {"S": "u1"} }
        }));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("Alice"));
    }

    #[test]
    fn test_scan() {
        let handler = DynamoDbHandler::new();
        handler.handle(make_req("CreateTable", json!({
            "TableName": "items",
            "KeySchema": [{"AttributeName": "pk", "KeyType": "HASH"}],
            "AttributeDefinitions": [{"AttributeName": "pk", "AttributeType": "S"}],
            "BillingMode": "PAY_PER_REQUEST"
        })));
        handler.handle(make_req("PutItem", json!({
            "TableName": "items",
            "Item": { "pk": {"S": "1"}, "data": {"S": "foo"} }
        })));
        let req = make_req("Scan", json!({"TableName": "items"}));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("foo"));
    }
}
