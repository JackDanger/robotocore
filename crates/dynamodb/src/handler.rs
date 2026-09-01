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
            "TransactWriteItems" => self.transact_write_items(&req),
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
            "CreateGlobalTable" => self.json_stub(&req, "GlobalTableDescription"),
            "DescribeEndpoints" => self.json_stub_list(&req, "Endpoints"),
            "DescribeTableReplicaAutoScaling" => self.json_stub(&req, "TableReplicaAutoScalingDescription"),
            "EnableKinesisStreamingDestination" => self.json_stub(&req, "StreamDescription"),
            "ExportTableToPointInTime" => self.json_stub(&req, "ExportDescription"),
            "ImportTable" => self.json_stub(&req, "TableDescription"),
            "ListContributorInsights" => self.json_stub_list(&req, "ContributorInsightSummaries"),
            "ListExports" => self.json_stub_list(&req, "Exports"),
            "ListGlobalTables" => self.json_stub_list(&req, "GlobalTables"),
            "ListImports" => self.json_stub_list(&req, "Imports"),
            "UpdateContributorInsights" => self.json_stub(&req, "ContributorInsights"),
            "UpdateGlobalTableSettings" => self.json_stub(&req, "GlobalTableSettingDescription"),
            "UpdateTableReplicaAutoScaling" => self.json_stub(&req, "TableReplicaAutoScalingDescription"),
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

        // Parse GSIs and LSIs
                let gsis: Vec<crate::models::IndexDefinition> = req
            .params
            .get("GlobalSecondaryIndexes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|g| {
                    Some(crate::models::IndexDefinition {
                        name: g.get("IndexName")?.as_str()?.to_string(),
                        key_schema: g.get("KeySchema")?.as_array()?.iter().filter_map(|k| {
                            Some(crate::models::KeySchema {
                                attribute_name: k.get("AttributeName")?.as_str()?.to_string(),
                                key_type: k.get("KeyType")?.as_str()?.to_string(),
                            })
                        }).collect(),
                        projection: g.get("Projection")?.get("ProjectionType")?.as_str()?.to_string(),
                    })
                }).collect()
            })
            .unwrap_or_default();
        let lsis: Vec<crate::models::IndexDefinition> = req
            .params
            .get("LocalSecondaryIndexes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|l| {
                    Some(crate::models::IndexDefinition {
                        name: l.get("IndexName")?.as_str()?.to_string(),
                        key_schema: l.get("KeySchema")?.as_array()?.iter().filter_map(|k| {
                            Some(crate::models::KeySchema {
                                attribute_name: k.get("AttributeName")?.as_str()?.to_string(),
                                key_type: k.get("KeyType")?.as_str()?.to_string(),
                            })
                        }).collect(),
                        projection: l.get("Projection")?.get("ProjectionType")?.as_str()?.to_string(),
                    })
                }).collect()
            })
            .unwrap_or_default();
        
        let table = {
            let mut t = Table::new(
                table_name.clone(),
                req.account,
                req.region.clone(),
                key_schema.clone(),
                attribute_definitions.clone(),
                billing_mode,
            );
            t.global_secondary_indexes = gsis;
            t.local_secondary_indexes = lsis;
            Arc::new(t)
        };
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
                    "LocalSecondaryIndexes": table.local_secondary_indexes.iter().map(|i| json!({
                        "IndexName": i.name,
                        "KeySchema": i.key_schema.iter().map(|k| json!({"AttributeName": k.attribute_name, "KeyType": k.key_type})).collect::<Vec<_>>(),
                        "Projection": {"ProjectionType": i.projection}
                    })).collect::<Vec<_>>(),
                    "GlobalSecondaryIndexes": table.global_secondary_indexes.iter().map(|i| json!({
                        "IndexName": i.name,
                        "KeySchema": i.key_schema.iter().map(|k| json!({"AttributeName": k.attribute_name, "KeyType": k.key_type})).collect::<Vec<_>>(),
                        "Projection": {"ProjectionType": i.projection}
                    })).collect::<Vec<_>>(),
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
                        "LocalSecondaryIndexes": table.local_secondary_indexes.iter().map(|i| json!({
                            "IndexName": i.name,
                            "KeySchema": i.key_schema.iter().map(|k| json!({"AttributeName": k.attribute_name, "KeyType": k.key_type})).collect::<Vec<_>>(),
                            "Projection": {"ProjectionType": i.projection}
                        })).collect::<Vec<_>>(),
                        "GlobalSecondaryIndexes": table.global_secondary_indexes.iter().map(|i| json!({
                            "IndexName": i.name,
                            "KeySchema": i.key_schema.iter().map(|k| json!({"AttributeName": k.attribute_name, "KeyType": k.key_type})).collect::<Vec<_>>(),
                            "Projection": {"ProjectionType": i.projection}
                        })).collect::<Vec<_>>(),
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

        let key_cond = req.params.get("KeyConditionExpression")
            .and_then(|v| v.as_str()).unwrap_or("");
        let expr_vals = req.params.get("ExpressionAttributeValues").cloned().unwrap_or(Value::Null);
        let expr_names: HashMap<String, String> = req.params.get("ExpressionAttributeNames")
            .and_then(|v| v.as_object())
            .map(|m| m.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
            .unwrap_or_default();
        let limit = req.params.get("Limit").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(1000);
        let filter_expr = req.params.get("FilterExpression")
            .and_then(|v| v.as_str()).unwrap_or("");
        let projection = req.params.get("ProjectionExpression")
            .and_then(|v| v.as_str()).unwrap_or("");

        let items = table.items.read();
        let mut result_items: Vec<Value> = Vec::new();
        let mut scanned = 0;

        for (_key, item) in items.iter() {
            scanned += 1;
            let item_json = serde_json::to_value(&item.attributes).unwrap_or(Value::Null);

            // Apply KeyConditionExpression
            if !key_cond.is_empty() {
                if !self.matches_key_condition(&item_json, key_cond, &expr_vals, &expr_names) {
                    continue;
                }
            }

            // Apply FilterExpression
            if !filter_expr.is_empty() {
                if !self.matches_filter(&item_json, filter_expr, &expr_vals, &expr_names) {
                    continue;
                }
            }

            // Apply projection
            let final_item = if !projection.is_empty() {
                let proj_attrs: Vec<String> = projection.split(',')
                    .map(|s| self.resolve_name(s.trim(), &expr_names)).collect();
                let mut proj = serde_json::Map::new();
                for attr in &proj_attrs {
                    if let Some(v) = item_json.get(attr) {
                        proj.insert(attr.clone(), v.clone());
                    }
                }
                Value::Object(proj)
            } else {
                item_json
            };

            result_items.push(final_item);
            if result_items.len() >= limit {
                break;
            }
        }

        // Sort by sort key if present
        let sort_key = table.key_schema.iter()
            .find(|k| k.key_type == "RANGE")
            .map(|k| k.attribute_name.clone());
        if let Some(sk) = sort_key {
            result_items.sort_by(|a, b| {
                let av = a.get(&sk).cloned().unwrap_or(Value::Null);
                let bv = b.get(&sk).cloned().unwrap_or(Value::Null);
                let as_str = av.to_string();
                let bs_str = bv.to_string();
                as_str.cmp(&bs_str)
            });
        }

        AwsResponse::json(200, json!({
            "Items": result_items,
            "Count": result_items.len(),
            "ScannedCount": scanned
        }))
    }

    fn matches_key_condition(&self, item: &Value, expr: &str, vals: &Value, names: &HashMap<String, String>) -> bool {
        let expr = expr.trim();

        // First, handle BETWEEN clauses (they contain " AND " inside)
        // Pattern: "attr BETWEEN :lo AND :hi"
        if let Some(between_pos) = expr.find(" BETWEEN ") {
            // Find the attr name (everything before BETWEEN, after last AND or start)
            let before = &expr[..between_pos];
            let attr_start = before.rfind(" AND ").map(|i| i + 4).unwrap_or(0);
            let attr = before[attr_start..].trim();
            let attr_resolved = self.resolve_name(attr, names);

            // After BETWEEN: ":lo AND :hi"
            let after = &expr[between_pos + 9..]; // skip " BETWEEN "
            let and_pos = after.rfind(" AND ").unwrap_or(0);
            let lo_ref = if and_pos > 0 { after[..and_pos].trim() } else { after.trim() };
            let hi_ref = if and_pos > 0 { after[and_pos + 5..].trim() } else { "" };

            let lo_val = vals.get(lo_ref).cloned().unwrap_or(Value::Null);
            let hi_val = vals.get(hi_ref).cloned().unwrap_or(Value::Null);
            let item_val = item.get(&attr_resolved).cloned().unwrap_or(Value::Null);
            let item_s = self.dyn_to_str(&item_val);
            let lo_s = self.dyn_to_str(&lo_val);
            let hi_s = self.dyn_to_str(&hi_val);
            if item_s < lo_s || item_s > hi_s {
                return false;
            }

            // Now handle the remaining conditions (before BETWEEN)
            if !before.is_empty() {
                let eq_parts: Vec<&str> = before.split(" AND ").map(|s| s.trim()).collect();
                for part in eq_parts {
                    if part.is_empty() || part.contains(" BETWEEN ") {
                        continue;
                    }
                    if let Some(eq_pos) = part.find('=') {
                        let p_attr = self.resolve_name(&part[..eq_pos].trim(), names);
                        let p_val_ref = part[eq_pos+1..].trim();
                        let p_val = vals.get(p_val_ref).cloned().unwrap_or(Value::Null);
                        let p_item_val = item.get(&p_attr).cloned().unwrap_or(Value::Null);
                        if p_item_val != p_val {
                            return false;
                        }
                    }
                }
            }
            return true;
        }

        // No BETWEEN - handle simple equality conditions
        let conditions: Vec<&str> = expr.split(" AND ").map(|s| s.trim()).collect();
        for cond in conditions {
            let cond = cond.trim();
            if cond.is_empty() {
                continue;
            }
            if let Some(eq_pos) = cond.find('=') {
                let attr = self.resolve_name(&cond[..eq_pos].trim(), names);
                let val_ref = cond[eq_pos+1..].trim();
                let val = vals.get(val_ref).cloned().unwrap_or(Value::Null);
                let item_val = item.get(&attr).cloned().unwrap_or(Value::Null);
                if item_val != val {
                    return false;
                }
            }
        }
        true
    }

    fn matches_filter(&self, item: &Value, expr: &str, vals: &Value, names: &HashMap<String, String>) -> bool {
        // Basic filter: "attr = :val" or "attr > :val" etc.
        let expr = expr.trim();
        // Handle simple "attr op :val" patterns
        for op in [">=", "<=", ">", "<", "="] {
            if expr.contains(op) {
                let parts: Vec<&str> = expr.split(op).collect();
                if parts.len() == 2 {
                    let attr = self.resolve_name(parts[0].trim(), names);
                    let val_ref = parts[1].trim();
                    let val = vals.get(val_ref).cloned().unwrap_or(Value::Null);
                    let item_val = item.get(&attr).cloned().unwrap_or(Value::Null);
                    let item_s = self.dyn_to_str(&item_val);
                    let val_s = self.dyn_to_str(&val);
                    match op {
                        "=" => { if item_s != val_s { return false; } }
                        ">" => { if item_s <= val_s { return false; } }
                        "<" => { if item_s >= val_s { return false; } }
                        ">=" => { if item_s < val_s { return false; } }
                        "<=" => { if item_s > val_s { return false; } }
                        _ => {}
                    }
                }
            }
        }
        true
    }

    fn dyn_to_str(&self, val: &Value) -> String {
        match val {
            Value::String(s) => s.clone(),
            Value::Object(m) => {
                if let Some(s) = m.get("S").and_then(|v| v.as_str()) {
                    s.to_string()
                } else if let Some(n) = m.get("N").and_then(|v| v.as_str()) {
                    n.to_string()
                } else {
                    val.to_string()
                }
            }
            _ => val.to_string(),
        }
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
        let attr_names: HashMap<String, String> = req.params.get("ExpressionAttributeNames")
            .and_then(|v| v.as_object())
            .map(|m| m.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
            .unwrap_or_default();
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
                self.apply_add_expr(&mut new_attrs, update_expr, &expression_values, &attr_names);
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
        self.apply_update_expr(&mut attrs, update_expr, &expression_values, &attr_names);

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

    fn resolve_name(&self, attr: &str, names: &HashMap<String, String>) -> String {
        if attr.starts_with('#') {
            names.get(attr).cloned().unwrap_or_else(|| attr.to_string())
        } else {
            attr.to_string()
        }
    }

    fn apply_update_expr(&self, attrs: &mut HashMap<String, Value>, expr: &str, values: &Value, attr_names: &HashMap<String, String>) {
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

                    let resolved = self.resolve_name(attr, attr_names);
                    attrs.insert(resolved, val);
                }
            }
        }

        // Handle ADD clause
        if let Some(add_pos) = expr.find("ADD") {
            let rest = expr[add_pos + 3..].trim();
            self.apply_add_expr(attrs, rest, values, attr_names);
        }

        // Handle REMOVE clause
        if let Some(remove_pos) = expr.find("REMOVE") {
            let rest = expr[remove_pos + 6..].trim();
            let remove_names: Vec<&str> = rest.split(",").map(|s| s.trim()).collect();
            for rn in remove_names {
                let resolved = self.resolve_name(rn, attr_names);
                attrs.remove(&resolved);
            }
        }
    }

    fn apply_add_expr(&self, attrs: &mut HashMap<String, Value>, expr: &str, values: &Value, attr_names: &HashMap<String, String>) {
        // ADD attr :val, attr2 :val2
        let parts: Vec<&str> = expr.split(",").map(|s| s.trim()).collect();
        for part in parts {
            let words: Vec<&str> = part.split_whitespace().collect();
            if words.len() == 2 {
                let attr = self.resolve_name(words[0], attr_names);
                let val_ref = words[1];
                let add_val = values.get(val_ref).cloned().unwrap_or(json!(0));
                let current = attrs.get(&attr).cloned().unwrap_or(json!({ "N": "0" }));
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
                let projection = keys_json.get("ProjectionExpression")
                    .and_then(|v| v.as_str())
                    .map(|s| s.split(", ").map(|p| p.trim().to_string()).collect::<Vec<_>>());
                let items = table.items.read();
                let mut found: Vec<Value> = Vec::new();
                for key_json in &keys {
                    let key_attrs: HashMap<String, Value> = key_json
                        .as_object()
                        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default();
                    for (_key_str, item) in items.iter() {
                        if key_attrs.iter().all(|(name, val)| item.attributes.get(name) == Some(val)) {
                            let attrs = &item.attributes;
                            let mut item_obj = serde_json::Map::new();
                            for (name, val) in attrs {
                                match &projection {
                                    Some(proj) if !proj.contains(name) => continue,
                                    _ => {}
                                }
                                if let Ok(val_json) = serde_json::to_value(val) {
                                    item_obj.insert(name.clone(), val_json);
                                }
                            }
                            found.push(Value::Object(item_obj));
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
        match state.get_table(&table_name) {
            Some(table) => {
                let enabled = *table.ttl_enabled.read();
                let status = if enabled { "ENABLED" } else { "DISABLED" };
                let mut desc = json!({
                    "TimeToLiveStatus": status
                });
                if enabled {
                    desc.as_object_mut().unwrap()
                        .insert("AttributeName".into(), json!(*table.ttl_attribute.read()));
                }
                AwsResponse::json(200, json!({ "TimeToLiveDescription": desc }))
            }
            None => AwsResponse::error(400, "ResourceNotFoundException", &format!("Table not found: {}", table_name))
        }
    }

    fn update_ttl(&self, req: &AwsRequest) -> AwsResponse {
        let table_name = req.params.get("TableName").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        let table = match state.get_table(table_name) {
            Some(t) => t,
            None => return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Requested resource not found: Table: {}", table_name)),
        };
        let ttl_spec = req.params.get("TimeToLiveSpecification").cloned().unwrap_or(Value::Null);
        let enabled = ttl_spec.get("Enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let attr = ttl_spec.get("AttributeName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        *table.ttl_enabled.write() = enabled;
        *table.ttl_attribute.write() = attr.clone();
        AwsResponse::json(200, json!({
            "TimeToLiveSpecification": {
                "TableName": table_name,
                "TimeToLive": { "Enabled": enabled, "AttributeName": attr }
            }
        }))
    }

    fn transact_write_items(&self, req: &AwsRequest) -> AwsResponse {
        let transact_items = req.params.get("TransactItems").cloned().unwrap_or(Value::Null);
        let items: Vec<Value> = transact_items.as_array().cloned().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut items_out: Vec<Value> = Vec::new();
        for item in &items {
            if let Some(put) = item.get("Put") {
                let table_name = put.get("TableName").and_then(|v| v.as_str()).unwrap_or("");
                let table = match state.get_table(table_name) {
                    Some(t) => t,
                    None => continue,
                };
                let item_val = put.get("Item").cloned().unwrap_or(Value::Null);
                let item_obj = item_val.as_object().cloned().unwrap_or_default();
                let attrs: std::collections::HashMap<String, Value> = item_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                let item = Arc::new(crate::models::Item::new(attrs));
                // Use the hash key as the item key
                let key_parts: Vec<String> = table.key_schema.iter()
                    .map(|k| {
                        let val = item_obj.get(&k.attribute_name)
                            .cloned()
                            .unwrap_or(Value::Null);
                        format!("{}:{}", k.attribute_name, self.dyn_to_str(&val))
                    })
                    .collect();
                let item_key = key_parts.join("|");
                table.items.write().insert(item_key, item);
                items_out.push(item_val);
            }
        }
        AwsResponse::json(200, json!({
            "ConsumedCapacity": null
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
