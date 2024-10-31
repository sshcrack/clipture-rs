import { Progress } from '@nextui-org/react';
import client from '../misc/client';
import { useEffect, useState } from 'react';

export default function BootstrapApp() {
    let [progress, setProgress] = useState(0)

    useEffect(() => {
        client.addSubscription(["bootstrap.pings"], {
            onData: data => {
                console.log(data)
                setProgress(data)
            }
        })
    }, [])

    return <>
        <p>Checking OBS install</p>
        <Progress value={progress} maxValue={100} />
    </>
}