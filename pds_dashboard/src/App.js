import React, { useEffect, useState } from "react";
import Ably from "ably";
import { Realtime } from "ably/browser/static/ably-commonjs.js";

const App = () => {
    const [logs, setLogs] = useState([]);
    const ably = new Realtime("YOUR_ABLY_API_KEY");

    useEffect(() => {
        const channel = ably.channels.get("logs");
        channel.subscribe("log", (message) => {
            setLogs((prevLogs) => [...prevLogs, message.data]);
        });

        return () => {
            channel.unsubscribe();
        };
    }, [ably]);

    return (
        <div className="App">
            <h1>Logs</h1>
            <ul>
                {logs.map((log, index) => (
                    <li key={index}>{log.message}</li>
                ))}
            </ul>
        </div>
    );
};

export default App;
