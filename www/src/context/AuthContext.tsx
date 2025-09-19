import React, { createContext, useContext, useEffect, useState } from 'react';
import { PublicClientApplication } from '@azure/msal-browser';
import type { AccountInfo, AuthenticationResult } from '@azure/msal-browser';
import { MsalProvider } from '@azure/msal-react';
import { loginRequest, createMsalConfig } from '../config/authConfig';

interface AuthContextType {
  isAuthenticated: boolean;
  account: AccountInfo | null;
  login: (tenantId?: string) => Promise<void>;
  logout: () => Promise<void>;
  getAccessToken: (scopes: string[]) => Promise<string | null>;
  loading: boolean;
  error: string | null;
  currentTenantId: string;
  setTenantId: (tenantId: string) => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

interface AuthProviderProps {
  children: React.ReactNode;
}

export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [account, setAccount] = useState<AccountInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [currentTenantId, setCurrentTenantId] = useState('common');
  const [msalInstance, setMsalInstance] = useState<PublicClientApplication | null>(null);

  // Initialize MSAL instance when tenant changes
  useEffect(() => {
    const initializeMsal = async () => {
      try {
        setLoading(true);
        
        // Create new MSAL instance with current tenant
        const config = createMsalConfig(currentTenantId);
        const newMsalInstance = new PublicClientApplication(config);
        
        // Initialize MSAL
        await newMsalInstance.initialize();
        
        setMsalInstance(newMsalInstance);
        
        // Handle redirect response
        const response = await newMsalInstance.handleRedirectPromise();
        if (response) {
          setAccount(response.account);
          setIsAuthenticated(true);
        } else {
          // Check if there are any accounts already signed in
          const accounts = newMsalInstance.getAllAccounts();
          if (accounts.length > 0) {
            setAccount(accounts[0]);
            setIsAuthenticated(true);
          }
        }
      } catch (err) {
        console.error('MSAL initialization error:', err);
        setError(err instanceof Error ? err.message : 'Authentication initialization failed');
      } finally {
        setLoading(false);
      }
    };

    initializeMsal();
  }, [currentTenantId]);

  const setTenantId = (tenantId: string) => {
    // Clear current authentication state when switching tenants
    setIsAuthenticated(false);
    setAccount(null);
    setError(null);
    setCurrentTenantId(tenantId);
  };

  const login = async (tenantId?: string): Promise<void> => {
    if (!msalInstance) {
      setError('MSAL instance not initialized');
      return;
    }

    try {
      setError(null);
      setLoading(true);
      
      // If a new tenant is provided, update it before login
      if (tenantId && tenantId !== currentTenantId) {
        setTenantId(tenantId);
        return; // The useEffect will handle the re-initialization and we'll need to call login again
      }
      
      const response = await msalInstance.loginPopup(loginRequest);
      
      setAccount(response.account);
      setIsAuthenticated(true);
    } catch (err) {
      console.error('Login error:', err);
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setLoading(false);
    }
  };

  const logout = async (): Promise<void> => {
    if (!msalInstance) {
      setError('MSAL instance not initialized');
      return;
    }

    try {
      setError(null);
      setLoading(true);
      
      await msalInstance.logoutPopup({
        postLogoutRedirectUri: createMsalConfig(currentTenantId).auth.postLogoutRedirectUri,
        account: account || undefined
      });
      
      setAccount(null);
      setIsAuthenticated(false);
    } catch (err) {
      console.error('Logout error:', err);
      setError(err instanceof Error ? err.message : 'Logout failed');
    } finally {
      setLoading(false);
    }
  };

  const getAccessToken = async (scopes: string[]): Promise<string | null> => {
    if (!msalInstance) {
      throw new Error('MSAL instance not initialized');
    }

    if (!account) {
      throw new Error('No account available for token acquisition');
    }

    try {
      setError(null);
      
      // Try to get token silently first
      const silentRequest = {
        scopes,
        account
      };
      
      let response: AuthenticationResult;
      
      try {
        response = await msalInstance.acquireTokenSilent(silentRequest);
      } catch (silentError) {
        console.log('Silent token acquisition failed, trying popup...', silentError);
        
        // If silent request fails, use popup
        response = await msalInstance.acquireTokenPopup({
          scopes,
          account
        });
      }
      
      return response.accessToken;
    } catch (err) {
      console.error('Token acquisition error:', err);
      setError(err instanceof Error ? err.message : 'Token acquisition failed');
      return null;
    }
  };

  const contextValue: AuthContextType = {
    isAuthenticated,
    account,
    login,
    logout,
    getAccessToken,
    loading,
    error,
    currentTenantId,
    setTenantId
  };

  // Don't render MsalProvider until we have an instance
  if (!msalInstance) {
    return (
      <AuthContext.Provider value={contextValue}>
        <div className="auth-loading">
          <div>Initializing authentication...</div>
        </div>
      </AuthContext.Provider>
    );
  }

  return (
    <AuthContext.Provider value={contextValue}>
      <MsalProvider instance={msalInstance}>
        {children}
      </MsalProvider>
    </AuthContext.Provider>
  );
};

export const useAuth = (): AuthContextType => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};