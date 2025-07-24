import re

# Read the file
with open('kincir/src/rabbitmq/tests.rs', 'r') as f:
    content = f.read()

# Pattern to match RabbitMQAckHandle::new calls with wrong parameter order
# The correct order is: message_id, topic, timestamp, delivery_count, delivery_tag
pattern = r'RabbitMQAckHandle::new\(\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,)]+)\s*\)'

def fix_constructor(match):
    message_id = match.group(1).strip()
    topic = match.group(2).strip()
    param3 = match.group(3).strip()
    param4 = match.group(4).strip()
    param5 = match.group(5).strip()
    
    # Check if this is already in correct order (timestamp should be SystemTime)
    if 'SystemTime' in param3 or 'timestamp' in param3:
        return match.group(0)  # Already correct
    
    # If param3 is a number (delivery_tag), we need to reorder
    if param3.isdigit() or 'delivery_tag' in param3 or 'as u64' in param3:
        # Old order: message_id, topic, delivery_tag, delivery_count, timestamp
        # New order: message_id, topic, timestamp, delivery_count, delivery_tag
        delivery_tag = param3
        delivery_count = param4
        timestamp = param5
        
        return f'RabbitMQAckHandle::new({message_id}, {topic}, {timestamp}, {delivery_count}, {delivery_tag})'
    
    return match.group(0)  # No change needed

# Apply the fix
fixed_content = re.sub(pattern, fix_constructor, content, flags=re.MULTILINE | re.DOTALL)

# Write back
with open('kincir/src/rabbitmq/tests.rs', 'w') as f:
    f.write(fixed_content)

print("Fixed RabbitMQ test constructors")
