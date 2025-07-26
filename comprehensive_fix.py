import re

# Fix MQTT tests
with open('kincir/src/mqtt/tests.rs', 'r') as f:
    mqtt_content = f.read()

# Fix all MQTT constructor calls - replace with correct order
mqtt_fixes = [
    # Fix the broken syntax first
    (r'SystemTime::now\(, (\d+), (QoS::\w+), (Some\(\d+\))\)', r'SystemTime::now(), \1, \2, \3'),
    
    # Fix remaining constructor calls with wrong parameter order
    (r'MQTTAckHandle::new\(\s*([^,]+),\s*([^,]+),\s*(QoS::\w+),\s*([^,]+),\s*(\d+),\s*([^,)]+)\s*\)', 
     r'MQTTAckHandle::new(\1, \2, \6, \5, \3, \4)'),
]

for pattern, replacement in mqtt_fixes:
    mqtt_content = re.sub(pattern, replacement, mqtt_content, flags=re.MULTILINE)

with open('kincir/src/mqtt/tests.rs', 'w') as f:
    f.write(mqtt_content)

# Fix RabbitMQ tests
with open('kincir/src/rabbitmq/tests.rs', 'r') as f:
    rabbitmq_content = f.read()

# Fix all RabbitMQ constructor calls
rabbitmq_fixes = [
    # Fix the broken syntax first
    (r'SystemTime::now\(, (\d+), (\d+)\)', r'SystemTime::now(), \1, \2'),
    
    # Fix remaining constructor calls with wrong parameter order
    (r'RabbitMQAckHandle::new\(\s*([^,]+),\s*([^,]+),\s*(\d+[^,]*),\s*(\d+),\s*([^,)]+)\s*\)', 
     r'RabbitMQAckHandle::new(\1, \2, \5, \4, \3)'),
]

for pattern, replacement in rabbitmq_fixes:
    rabbitmq_content = re.sub(pattern, replacement, rabbitmq_content, flags=re.MULTILINE)

with open('kincir/src/rabbitmq/tests.rs', 'w') as f:
    f.write(rabbitmq_content)

print("Applied comprehensive fixes")
