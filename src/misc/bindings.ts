// This file was generated by [rspc](https://github.com/oscartbeaumont/rspc). Do not edit this file manually.

export type Procedures = {
    queries: 
        { key: "auth.is_logged_in", input: never, result: boolean } | 
        { key: "auth.open_auth_window", input: never, result: null } | 
        { key: "bootstrap.show_main", input: never, result: null },
    mutations: 
        { key: "auth.sign_in", input: never, result: null } | 
        { key: "auth.sign_out", input: never, result: null },
    subscriptions: 
        { key: "bootstrap.initialize", input: never, result: BootstrapStatus }
};

/**
 * 
 */
export type BootstrapStatus = { Error: string } | { Progress: [number, string] } | "Done"
