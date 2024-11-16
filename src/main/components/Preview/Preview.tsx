import { DetailedHTMLProps, HTMLAttributes, useEffect, useRef } from 'react';
import client from '../../../misc/client';
import { getCurrentWebview } from '@tauri-apps/api/webview';

export default function Preview(props: DetailedHTMLProps<HTMLAttributes<HTMLDivElement>, HTMLDivElement>) {
    const ref = useRef<HTMLDivElement>(null)

    useEffect(() => {
        if (!ref.current)
            return

        const div = ref.current;
        const label = getCurrentWebview().label
        const rect = div.getBoundingClientRect()

        let id = -1
        const resize = new ResizeObserver(() => {
            if (id === -1)
                return;

            const rect = div.getBoundingClientRect()
            client.mutation(["obs.preview.set_size", {
                id,
                width: rect.width,
                height: rect.height
            }])
        })

        const reposition = (_event: Event) => {
            if (id === -1)
                return;

            const rect = div.getBoundingClientRect()
            client.mutation(["obs.preview.set_pos", {
                id,
                x: rect.x,
                y: rect.y
            }])
        }

        document.addEventListener("scroll", reposition)


        resize.observe(ref.current)

        let destroyOld = false
        const timeoutId = setTimeout(() => {
            console.log(`Creating display with label '${label}'`)
            client.mutation(["obs.preview.create", {
                window_label: label,
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                background_color: 0
            }]).then(id => {
                id = id

                // If this response wasn't fast enough and it should be destroyed immediately again
                if (destroyOld) {
                    client.mutation(["obs.preview.destroy", id])
                        .then(() => console.log("Display has been destroyed with id", id))
                        .catch(e => console.error("Couldn't destroy display with id", id, e))
                }
            })
                .catch(e => console.error("Couldn't create display", e))
        }, 200)

        return () => {
            clearTimeout(timeoutId)
            resize.disconnect()
            document.removeEventListener("scroll", reposition)
            destroyOld = true
            if (id !== -1) {
                client.mutation(["obs.preview.destroy", id])
                    .then(() => console.log("Display has been destroyed with id", id))
                    .catch(e => console.error("Couldn't destroy display with id", id, e))
            }
        }
    }, [])

    return <div {...props} ref={ref}>
        <div className="w-full h-full bg-gray-800 flex items-center justify-center">
            <div className="text-white text-4xl">Preview</div>
        </div>
    </div>
}