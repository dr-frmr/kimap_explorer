import React, { useState } from 'react';

interface DataKeyElementProps {
    dataKey: string;
    dataValue: string;
}

const DataKeyElement: React.FC<DataKeyElementProps> = ({ dataKey, dataValue }) => {
    const [valueVisible, setValueVisible] = useState(false);

    const toggleValue = () => setValueVisible(!valueVisible);

    const tryParseUtf8 = (hexString: string) => {
        try {
            const cleanHex = hexString.startsWith('0x') ? hexString.slice(2) : hexString;
            const hexPairs = cleanHex.match(/.{1,2}/g);
            if (!hexPairs) return hexString;
            const bytes = new Uint8Array(hexPairs.map(byte => parseInt(byte, 16)));
            const decoded = new TextDecoder('utf-8').decode(bytes);
            if (/^[\x20-\x7E]*$/.test(decoded)) {
                return decoded;
            }
        } catch (error) {
            console.warn('Failed to parse as UTF-8:', error);
        }
        return hexString;
    };

    return (
        <div className="data-key">
            {dataKey}
            <button className="info-button" onClick={toggleValue}>📄</button>
            {valueVisible && (
                <div className="info-container">
                    {tryParseUtf8(dataValue)}
                </div>
            )}
        </div>
    );
};

export default DataKeyElement;