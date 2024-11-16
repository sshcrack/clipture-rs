import { DetailedHTMLProps, HTMLAttributes, useEffect, useRef } from 'react';

export default function Preview(props: DetailedHTMLProps<HTMLAttributes<HTMLDivElement>, HTMLDivElement>) {
    const ref = useRef<HTMLDivElement>(null)

    useEffect(() => {
        if (!ref.current)
            return

        
    }, [ref])

    return <div {...props} ref={ref}>
        <div className="w-full h-full bg-gray-800 flex items-center justify-center">
            <div className="text-white text-4xl">Preview</div>
        </div>
    </div>
}