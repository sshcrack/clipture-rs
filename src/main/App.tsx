import { Button } from '@nextui-org/react';
import { Toaster } from 'react-hot-toast';
import "./App.css";
import client from '../misc/client';
import { relaunch } from '@tauri-apps/plugin-process';
import { restoreStateCurrent, StateFlags} from "@tauri-apps/plugin-window-state"
import { useEffect } from 'react';

// Maybe use ParkUI?
function App() {
  useEffect(() => {
    console.log("Restoring state")
    restoreStateCurrent(StateFlags.POSITION | StateFlags.SIZE | StateFlags.MAXIMIZED)
  }, [])

return <>
    <Button onClick={async () => {
      client.mutation(["auth.sign_out"])
        .then(() => relaunch())
    }}>Sign Out</Button>
    <Toaster />
  </>;
}

export default App;
