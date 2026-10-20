// Development entry only; the production companion build never imports this.
import {createRoot} from 'react-dom/client'
import {CompanionStudio} from './features/companion/studio/CompanionStudio'
import './features/companion/companion.css'
if(import.meta.env.DEV)createRoot(document.getElementById('root')!).render(<CompanionStudio/>)
