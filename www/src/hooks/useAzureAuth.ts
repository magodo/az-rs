import { useAuth } from '../context/AuthContext';
import { azureScopes } from '../config/authConfig';

/**
 * Custom hook for Azure authentication operations
 */
export const useAzureAuth = () => {
  const auth = useAuth();

  /**
   * Get access token for Azure Resource Manager API
   * This token can be used to manage Azure resources
   */
  const getAzureManagementToken = async (): Promise<string | null> => {
    return auth.getAccessToken(azureScopes.management);
  };

  /**
   * Get access token for custom scopes
   * @param scopes - Array of scope strings
   */
  const getCustomToken = async (scopes: string[]): Promise<string | null> => {
    return auth.getAccessToken(scopes);
  };

  return {
    ...auth,
    getAzureManagementToken,
    getCustomToken,
  };
};

/**
 * Custom hook for checking authentication status
 */
export const useAuthStatus = () => {
  const { isAuthenticated, account, loading, error, currentTenantId } = useAuth();

  return {
    isAuthenticated,
    account,
    loading,
    error,
    currentTenantId,
    isReady: !loading,
    hasError: !!error,
    userName: account?.name || account?.username,
    userEmail: account?.username,
  };
};