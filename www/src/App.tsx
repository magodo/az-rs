import './App.css'
import { AuthProvider } from './context/AuthContext'
import { AuthenticatedContent, AuthButton, UserProfile } from './components/AuthComponents'
import { AzureCLIInterface } from './components/AzureCLIInterface'

function AppContent() {

  return (
    <>
      <header className="app-header">
        <h1>Azure CLI on Browser</h1>
        <AuthButton />
      </header>

      <main className="app-main">
        <AuthenticatedContent
          fallback={
            <div className="welcome-message">
              <h2>Welcome to Azure CLI Browser</h2>
              <p>Sign in with your Microsoft account to access Azure resources and manage your cloud infrastructure.</p>
            </div>
          }
        >
          <div className="authenticated-content">
            <UserProfile />
            
            <AzureCLIInterface />

          </div>
        </AuthenticatedContent>
      </main>
    </>
  )
}

function App() {
  return (
    <AuthProvider>
      <AppContent />
    </AuthProvider>
  )
}

export default App
