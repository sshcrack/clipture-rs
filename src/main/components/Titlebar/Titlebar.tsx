import { VscChromeClose, VscChromeMaximize, VscChromeMinimize, VscChromeRestore } from "react-icons/vsc";
import { getCurrentWindow } from '@tauri-apps/api/window';
import "./titlebar.css"
import { useEffect, useMemo, useState } from 'react';

export default function Titlebar() {
    const appWindow = useMemo(() => getCurrentWindow(), [])
    const [isMaximized, setIsMaximized] = useState(false)

    useEffect(() => {
        appWindow.isMaximized().then(setIsMaximized)
    }, [appWindow])
    return <>
        <div data-tauri-drag-region className="titlebar bg-slate-900 z-50">
            <div onClick={() => appWindow.minimize()} className="titlebar-button color-white hover:bg-white/[0.25] hover:ease-in duration-100" id="titlebar-minimize">
                <VscChromeMinimize />
            </div>
            <div onClick={() => {
                appWindow.toggleMaximize()
                    .then(() => appWindow.isMaximized())
                    .then(setIsMaximized)
            }} className="titlebar-button hover:bg-white/[0.25] hover:ease-in duration-100" id="titlebar-maximize">
                {isMaximized ? <VscChromeRestore /> : <VscChromeMaximize />}
            </div>
            <div onClick={() => appWindow.close()} className="titlebar-button hover:bg-red-500/[0.5] hover:ease-in duration-100" id="titlebar-close">
                <VscChromeClose />
            </div>
        </div>
    </>
}