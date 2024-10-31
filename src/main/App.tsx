import { Button } from '@nextui-org/react';
import "./App.css";
import client from '../misc/client';
import toast, { Toaster } from 'react-hot-toast';

// Maybe use ParkUI?
function App() {
  return <>
    <Button onClick={async () => {
      const e = await client.query(["display_toggle"])
      toast.success(`R2F: ${e}`)
    }}>Greet button</Button>
    <Toaster />
  </>;
}

export default App;
