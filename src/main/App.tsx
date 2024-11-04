import { Button } from '@nextui-org/react';
import { Toaster } from 'react-hot-toast';
import "./App.css";
import client from '../misc/client';
import { relaunch } from '@tauri-apps/plugin-process';

// Maybe use ParkUI?
function App() {
  return <>
    <Button onClick={async () => {
      client.mutation(["auth.sign_out"])
        .then(() => relaunch())
    }}>Sign Out</Button>
    <Toaster />
  </>;
}

export default App;
