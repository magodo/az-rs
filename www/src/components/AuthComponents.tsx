import React, { useState } from 'react';
import { useAuthStatus, useAzureAuth } from '../hooks/useAzureAuth';
import { TenantInput } from './TenantInput';

interface AuthenticatedContentProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

/**
 * Component that only renders children if user is authenticated
 * Shows login UI if not authenticated
 */
export const AuthenticatedContent: React.FC<AuthenticatedContentProps> = ({ 
  children, 
  fallback 
}) => {
  const { isAuthenticated, loading } = useAuthStatus();

  if (loading) {
    return (
      <div className="auth-loading">
        <div>Loading authentication...</div>
      </div>
    );
  }

  if (!isAuthenticated) {
    return fallback ? <>{fallback}</> : <LoginPrompt />;
  }

  return <>{children}</>;
};

/**
 * Login/Logout button component with tenant selection
 */
export const AuthButton: React.FC = () => {
  const { isAuthenticated, loading, error } = useAuthStatus();
  const { login, logout, currentTenantId } = useAzureAuth();
  const [selectedTenantId, setSelectedTenantId] = useState(currentTenantId);

  const handleAuthAction = async () => {
    if (isAuthenticated) {
      await logout();
    } else {
      await login(selectedTenantId);
    }
  };

  const handleTenantChange = (tenantId: string) => {
    setSelectedTenantId(tenantId);
    // If user is already authenticated and changes tenant, we should logout first
    if (isAuthenticated && tenantId !== currentTenantId) {
      logout();
    }
  };

  if (isAuthenticated) {
    return (
      <div className="auth-button-container authenticated">
        <button 
          onClick={handleAuthAction}
          disabled={loading}
          className="auth-button logout"
        >
          {loading ? 'Loading...' : 'Sign Out'}
        </button>
        
        {error && (
          <div className="auth-error">
            Error: {error}
          </div>
        )}
        
        <div className="tenant-info">
          <small>Current tenant: {currentTenantId}</small>
        </div>
      </div>
    );
  }

  return (
    <div className="auth-button-container unauthenticated">
      <div className="auth-horizontal-layout">
        <div className="tenant-input-wrapper">
          <TenantInput
            onTenantChange={handleTenantChange}
            disabled={loading}
            initialValue={selectedTenantId}
          />
        </div>
        
        <div className="auth-action-wrapper">
          <button 
            onClick={handleAuthAction}
            disabled={loading}
            className="auth-button login"
          >
            {loading ? 'Loading...' : 'Sign In with Microsoft'}
          </button>
          
          {error && (
            <div className="auth-error">
              Error: {error}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

/**
 * User profile display component
 */
export const UserProfile: React.FC = () => {
  const { account, userName, userEmail } = useAuthStatus();

  if (!account) {
    return null;
  }

  return (
    <div className="user-profile">
      <div className="user-info">
        <h3>Welcome, {userName}!</h3>
        <p>Email: {userEmail}</p>
      </div>
    </div>
  );
};

/**
 * Login prompt component with tenant selection
 */
export const LoginPrompt: React.FC = () => {
  return (
    <div className="login-prompt">
      <h2>Welcome to Azure CLI Browser</h2>
      <p>Access Azure resources and manage your cloud infrastructure from your browser.</p>
      <AuthButton />
    </div>
  );
};
