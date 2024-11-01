import { Button } from '@nextui-org/react';
import "./App.css";
import client from '../misc/client';
import toast, { Toaster } from 'react-hot-toast';

// Maybe use ParkUI?
function App() {
  return <>
    <Button onClick={async () => {
    }}>Greet button</Button>
    <Toaster />
  </>;
}

export default App;
