import { Button, Progress } from '@nextui-org/react';
import { useEffect, useState } from 'react';
import toast, { Toaster } from 'react-hot-toast';
import client from '../misc/client';
import { FaDiscord } from 'react-icons/fa';
import "./App.css"
import { motion } from 'framer-motion';
import { useAnimate } from 'framer-motion/mini';

export default function BootstrapApp() {
    const [scope, animate] = useAnimate()
    const [progress, setProgress] = useState(0)
    const [done, setDone] = useState(false)
    const [status, setStatus] = useState("Checking OBS install")
    const [showLogin, setShowLogin] = useState(false)
    const [isLoggingIn, setLoggingIn] = useState(false)
    const [update, setUpdate] = useState(0)

    useEffect(() => {
        const unsubscribe = client.addSubscription(["bootstrap.initialize"], {
            onData: data => {
                if (data == "Done") {
                    console.log("Received done")
                    setProgress(1)
                    setStatus("Done")
                    setDone(true)
                    setUpdate(Math.random())
                }

                if (!(typeof data == "object"))
                    return

                if ("Progress" in data) {
                    const [percentage, message] = data["Progress"]
                    setProgress(percentage)
                    setStatus(message)
                }

                if ("Error" in data) {
                    console.error("Received error", data["Error"])
                    toast.error(data["Error"])
                }
            }
        })

        return () => unsubscribe()
    }, [])

    useEffect(() => {
        if (!done)
            return;

        client.query(["auth.is_logged_in"])
            .then(logged_in => {
                if (!logged_in) {
                    console.log("Showing login screen")
                    setShowLogin(true)
                } else {
                    console.log("Opening main screen")
                    client.query(["bootstrap.show_main"])
                }
            })
            .catch(e => {
                console.error("Error while checking if logged in", e)
                toast.error("Error while checking if logged in")

                setTimeout(() => {
                    setUpdate(Math.random())
                }, 5000)
            })

    }, [done, update])

    useEffect(() => {
        if (!isLoggingIn)
            return

        animate(scope.current, { opacity: 1, transform: "translateY(0px)" }, { duration: 0.6 })
    }, [scope, isLoggingIn])

    if (!done || !showLogin)
        return <div className='flex flex-col gap-6 justify-center pl-5 pr-5 items-center h-full'>
            <p className='text-lg top-2'>Initializing...</p>
            <Toaster />
            <Progress label={status} classNames={{ label: "truncate" }} showValueLabel={true} value={progress} maxValue={1} />
        </div>

    return <div className='flex flex-col gap-6 justify-center pl-5 pr-5 items-center h-full'>
        <Button startContent={<FaDiscord />} onClick={() => {
            setLoggingIn(true)
            client.mutation(["auth.sign_in"])
                .then(() => {
                    console.log("Done logging in. Showing main screen")
                    client.query(["bootstrap.show_main"])
                })
                .catch(e => {
                    if (typeof e == "object" && typeof e["message"] === "string") {
                        const msg = (e["message"] as string).toLowerCase()
                        if (msg.includes("[already_log]")) {
                            setTimeout(() => {
                                setLoggingIn(true)
                            }, 50)
                            return
                        }
                    }

                    console.error(e)
                    toast.error(`Couldn't  log in. Please try again`)
                })
                .finally(() => {
                    setLoggingIn(false)
                })
        }} isLoading={isLoggingIn} style={{ backgroundColor: "#7289da", zIndex: 10 }}>Sign in using discord</Button>
        <Toaster />
        <motion.p
            ref={scope}
            initial={{ opacity: 0, transform: "translateY(-60px)", zIndex: 0 }}
        >
            Didn't work? Click <span onClick={() => client.query(["auth.open_auth_window"])} className='helpText'>here</span></motion.p>
    </div>
}