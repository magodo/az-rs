import React, { useState } from 'react';

interface TenantInputProps {
  onTenantChange: (tenantId: string) => void;
  disabled?: boolean;
  initialValue?: string;
}

/**
 * Component for tenant ID input with validation
 */
export const TenantInput: React.FC<TenantInputProps> = ({ 
  onTenantChange, 
  disabled = false,
  initialValue = 'common'
}) => {
  const [tenantId, setTenantId] = useState(initialValue);
  const [isValid, setIsValid] = useState(true);

  const validateTenantId = (value: string): boolean => {
    // Allow 'common', 'organizations', 'consumers', or a valid GUID
    const guidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    const validSpecialValues = ['common', 'organizations', 'consumers'];
    
    return validSpecialValues.includes(value.toLowerCase()) || guidRegex.test(value);
  };

  const handleTenantChange = (value: string) => {
    setTenantId(value);
    const valid = validateTenantId(value);
    setIsValid(valid);
    
    if (valid) {
      onTenantChange(value);
    }
  };

  return (
    <div className="tenant-input-container">
      <label htmlFor="tenant-input" className="tenant-input-label">
        Azure Tenant ID:
      </label>
      <input
        id="tenant-input"
        type="text"
        value={tenantId}
        onChange={(e) => handleTenantChange(e.target.value)}
        disabled={disabled}
        className={`tenant-input ${!isValid ? 'invalid' : ''}`}
        placeholder="common"
      />
      {!isValid && (
        <div className="tenant-input-error">
          Please enter 'common', 'organizations', 'consumers', or a valid tenant GUID
        </div>
      )}
      <div className="tenant-input-help">
        <small>
          Use 'common' for multi-tenant access, or your specific tenant GUID for single-tenant.
        </small>
      </div>
    </div>
  );
};