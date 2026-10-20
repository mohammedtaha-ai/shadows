import {isDesktop} from '../../desktop/runtime'
import {CompanionSettings} from './companion-settings'
import {CompanionControls} from './companion-controls'
import {ConnectionSettings} from './connection-settings'

export function NativeSettings(){
  return isDesktop()?<NativeSections/>:null
}
function NativeSections(){
  return <div className="space-y-8 [&_button]:rounded [&_button]:border [&_button]:px-3 [&_button]:py-2 [&_.actions]:flex [&_.actions]:flex-wrap [&_.actions]:gap-2 [&_h2]:mb-3 [&_h2]:font-medium">
    <section aria-label="Desktop character"><h2>AYLUL character</h2><CompanionControls/><CompanionSettings/></section>
    <ConnectionSettings/>
  </div>
}
