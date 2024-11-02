import { Progress } from '@nextui-org/react';
import { useEffect, useState } from 'react';
import toast, { Toaster } from 'react-hot-toast';
import client from '../misc/client';

export default function BootstrapApp() {
    let [progress, setProgress] = useState(0)
    let [status, setStatus] = useState("Checking OBS install")

    useEffect(() => {
        console.log("Subscription")
        const unsubscribe = client.addSubscription(["bootstrap.initialize"], {
            onData: data => {
                if (data == "Done") {
                    setProgress(1)
                    setStatus("Done")
                }

                if (!(typeof data == "object"))
                    return

                if ("Progress" in data) {
                    const [percentage, message] = data["Progress"]
                    setProgress(percentage)
                    setStatus(message)
                }

                if ("Error" in data) {
                    toast.error(data["Error"])
                }
            }
        })

        return () => {
            console.log("Unsubscribing")
            unsubscribe()
        };
    }, [])

    return <div className='flex flex-col gap-6 justify-center pl-5 pr-5 items-center h-full'>
        <p className='text-lg top-2'>Initializing...</p>
        <Toaster />
        <Progress label={status} classNames={{ label: "truncate" }} showValueLabel={true} value={progress} maxValue={1} />
    </div>
}