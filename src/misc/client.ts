import { TauriTransport } from '@rspc/tauri';
import { Procedures } from './bindings';
import { createClient } from '@rspc/client';

const client = createClient<Procedures>({
    transport: new TauriTransport(),
});

export default client