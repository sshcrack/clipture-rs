import { CheckCircle2, X } from "lucide-react";

export interface ToastMessage {
  id: string;
  title: string;
  body: string;
}

export function ToastHost({ toasts, dismiss }: { toasts: ToastMessage[]; dismiss: (id: string) => void }) {
  return <div className="toast-host" aria-live="polite" aria-atomic="false">
    {toasts.map(toast => <div className="toast" key={toast.id} role="status">
      <span className="toast-icon"><CheckCircle2 size={16}/></span>
      <div><strong>{toast.title}</strong><span>{toast.body}</span></div>
      <button onClick={() => dismiss(toast.id)} aria-label="Dismiss notification"><X size={14}/></button>
    </div>)}
  </div>;
}
