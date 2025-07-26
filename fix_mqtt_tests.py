import re

# Read the file
with open('kincir/src/mqtt/tests.rs', 'r') as f:
    content = f.read()

# Pattern to match MQTTAckHandle::new calls with wrong parameter order
# The correct order is: message_id, topic, timestamp, delivery_count, qos, packet_id
pattern = r'MQTTAckHandle::new\(\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,)]+)\s*\)'

def fix_constructor(match):
    message_id = match.group(1).strip()
    topic = match.group(2).strip()
    param3 = match.group(3).strip()
    param4 = match.group(4).strip()
    param5 = match.group(5).strip()
    param6 = match.group(6).strip()
    
    # Check if this is already in correct order (timestamp should be SystemTime)
    if 'SystemTime' in param3 or 'timestamp' in param3:
        return match.group(0)  # Already correct
    
    # If param3 is QoS, we need to reorder
    if 'QoS::' in param3:
        # Old order: message_id, topic, qos, packet_id, delivery_count, timestamp
        # New order: message_id, topic, timestamp, delivery_count, qos, packet_id
        qos = param3
        packet_id = param4
        delivery_count = param5
        timestamp = param6
        
        return f'MQTTAckHandle::new({message_id}, {topic}, {timestamp}, {delivery_count}, {qos}, {packet_id})'
    
    return match.group(0)  # No change needed

# Apply the fix
fixed_content = re.sub(pattern, fix_constructor, content, flags=re.MULTILINE | re.DOTALL)

# Write back
with open('kincir/src/mqtt/tests.rs', 'w') as f:
    f.write(fixed_content)

print("Fixed MQTT test constructors")
