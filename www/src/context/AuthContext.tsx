import React, { createContext, useContext, useEffect, useState } from 'react';
import { PublicClientApplication } from '@azure/msal-browser';
import type { AccountInfo, AuthenticationResult } from '@azure/msal-browser';
import { MsalProvider } from '@azure/msal-react';
import { loginRequest, msalConfig } from '../config/authConfig';

interface AuthContextType {
  isAuthenticated: boolean;
  account: AccountInfo | null;
  login: () => Promise<void>;
  logout: () => Promise<void>;
  getAccessToken: (scopes: string[]) => Promise<string | null>;
  loading: boolean;
  error: string | null;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

// Create MSAL instance
export const msalInstance = new PublicClientApplication(msalConfig);

interface AuthProviderProps {
  children: React.ReactNode;
}

export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [account, setAccount] = useState<AccountInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const initializeMsal = async () => {
      try {
        // Initialize MSAL
        await msalInstance.initialize();
        
        // Handle redirect response
        const response = await msalInstance.handleRedirectPromise();
        if (response) {
          setAccount(response.account);
          setIsAuthenticated(true);
        } else {
          // Check if there are any accounts already signed in
          const accounts = msalInstance.getAllAccounts();
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
  }, []);

  const login = async (): Promise<void> => {
    try {
      setError(null);
      setLoading(true);
      
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
    try {
      setError(null);
      setLoading(true);
      
      await msalInstance.logoutPopup({
        postLogoutRedirectUri: msalConfig.auth.postLogoutRedirectUri,
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
    error
  };

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