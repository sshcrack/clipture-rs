import { Button } from '@nextui-org/react';
import { Toaster } from 'react-hot-toast';
import "./App.css";
import client from '../misc/client';
import { relaunch } from '@tauri-apps/plugin-process';
import { restoreStateCurrent, StateFlags } from "@tauri-apps/plugin-window-state"
import { useEffect } from 'react';
import Titlebar from './components/Titlebar/Titlebar';
import Preview from './components/Preview/Preview';

// Maybe use ParkUI?
function App() {
  useEffect(() => {
    client.addSubscription(["game_detect.game_open"], {
      onData: (data) => {
        console.log(data)
      },
      onError(err) {
        console.error(err)
      },
      onStarted() {
        console.log("Subscription started")
      },
    })

    restoreStateCurrent(StateFlags.POSITION | StateFlags.SIZE | StateFlags.MAXIMIZED)
  }, [])

  return <div className='w-full h-full'>
    <Titlebar />
    <Toaster />
    <div className='h-full w-full flex flex-col' style={{ paddingTop: "30px" }}>
      <div className='flex-shrink-1'>
        <Button onClick={async () => {
          client.mutation(["auth.sign_out"])
            .then(() => relaunch())
        }}>Sign Out</Button>
      </div>
      <Preview className='flex-1' />
    </div>
  </div>;
}

export default App;
