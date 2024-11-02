import { Button } from '@nextui-org/react';
import { Toaster } from 'react-hot-toast';
import "./App.css";

// Maybe use ParkUI?
function App() {
  return <>
    <Button onClick={async () => {
    }}>Greet button</Button>
    <Toaster />
  </>;
}

export default App;
