import type { Configuration, PopupRequest } from "@azure/msal-browser";

/**
 * Configuration object to be passed to MSAL instance on creation.
 * For a full list of MSAL configuration parameters, visit:
 * https://github.com/AzureAD/microsoft-authentication-library-for-js/blob/dev/lib/msal-browser/docs/configuration.md
 */
export const msalConfig: Configuration = {
    auth: {
        clientId: import.meta.env.VITE_CLIENT_ID || "your-client-id-here", // This is the ONLY mandatory field
        authority: import.meta.env.VITE_AUTHORITY || "https://login.microsoftonline.com/common", // Defaults to "https://login.microsoftonline.com/common"
        redirectUri: import.meta.env.VITE_REDIRECT_URI || window.location.origin, // Points to window.location.origin by default
        postLogoutRedirectUri: import.meta.env.VITE_POST_LOGOUT_REDIRECT_URI || window.location.origin, // Indicates the page to navigate after logout.
    },
    cache: {
        cacheLocation: "sessionStorage", // Configures cache location. "sessionStorage" is more secure, but "localStorage" gives you SSO between tabs.
        storeAuthStateInCookie: false, // Set this to "true" if you are having issues on IE11 or Edge
    },
    system: {
        loggerOptions: {
            loggerCallback: (level, message, containsPii) => {
                if (containsPii) {
                    return;
                }
                switch (level) {
                    case 0:
                        console.error(message);
                        return;
                    case 1:
                        console.warn(message);
                        return;
                    case 2:
                        console.info(message);
                        return;
                    case 3:
                        console.debug(message);
                        return;
                    default:
                        return;
                }
            },
        },
    },
};

/**
 * Creates MSAL configuration with a specific tenant ID
 * @param tenantId - The tenant ID to use (defaults to 'common')
 */
export const createMsalConfig = (tenantId: string = 'common'): Configuration => {
    return {
        ...msalConfig,
        auth: {
            ...msalConfig.auth,
            authority: `https://login.microsoftonline.com/${tenantId}`,
        },
    };
};

/**
 * Scopes you add here will be prompted for user consent during sign-in.
 * By default, MSAL.js will add OIDC scopes (openid, profile, email) to any login request.
 * For more information about OIDC scopes, visit: 
 * https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-permissions-and-consent#openid-connect-scopes
 */
export const loginRequest: PopupRequest = {
    scopes: ["https://management.azure.com/user_impersonation"],
    prompt: 'select_account'
};

/**
 * Add here the scopes to request when obtaining an access token for MS Graph API. For more information, see:
 * https://github.com/AzureAD/microsoft-authentication-library-for-js/blob/dev/lib/msal-browser/docs/resources-and-scopes.md
 */
export const graphConfig = {
    graphMeEndpoint: "https://graph.microsoft.com/v1.0/me",
};

/**
 * Add here the scopes for different Azure services you want to access
 */
export const azureScopes = {
    // Azure Resource Manager API
    management: ["https://management.azure.com/user_impersonation"],

};

/**
 * Token request configuration for different Azure services
 */
export const tokenRequests = {
    azureManagement: {
        scopes: azureScopes.management,
        account: null, // Will be set dynamically
    }
};