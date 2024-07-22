import { ConnectButton } from '@rainbow-me/rainbowkit';
import KimapExplorer from './components/KimapExplorer';

function App() {
  return (
    <div className="app-container">
      <header className="app-header">
        <h1 className="app-title">KIMAP Explorer</h1>
        <ConnectButton />
      </header>
      <KimapExplorer />
    </div>
  );
}

export default App;