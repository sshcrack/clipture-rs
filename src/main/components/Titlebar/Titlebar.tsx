import { FaWindowClose, FaWindowMaximize, FaWindowMinimize } from 'react-icons/fa'
import "./titlebar.css"

export default function Titlebar() {
    return <div data-tauri-drag-region className="titlebar bg-gradient-to-r from-purple-400 via-blue-400 to-blue-200">
        <div className="titlebar-button color-white hover:bg-purple-300" id="titlebar-minimize">
            <FaWindowMinimize />
        </div>
        <div className="titlebar-button" id="titlebar-maximize">
            <FaWindowMaximize />
        </div>
        <div className="titlebar-button" id="titlebar-close">
            <FaWindowClose />
        </div>
    </div>
}