import { createRoot } from 'react-dom/client';
import { App } from './App';
import './style.css';
import { installBrowserPolicy } from './browserPolicy';
installBrowserPolicy(import.meta.env.PROD);
createRoot(document.getElementById('root')!).render(<App/>);
