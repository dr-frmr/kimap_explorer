import React, { useState, useEffect } from 'react';
import { useAccount, useWriteContract } from 'wagmi';
import { useConnectModal } from '@rainbow-me/rainbowkit';
import { useAddRecentTransaction } from '@rainbow-me/rainbowkit';


import { fetchNodeInfo, mintFunction, noteFunction } from '../abis/helpers';
import { KINOMAP, mechAbi } from '../abis';

interface InfoContainerProps {
    hash: string;
    refetchNode: () => void;
}

interface Info {
    owner: string;
    tba: string;
    data_hex: string;
}

const InfoContainer: React.FC<InfoContainerProps> = ({ hash, refetchNode }) => {
    const [info, setInfo] = useState<Info | null>(null);
    const [key, setKey] = useState('');
    const [value, setValue] = useState('');
    const [subname, setSubname] = useState('');

    const { address } = useAccount();
    const { openConnectModal } = useConnectModal();
    const addRecentTransaction = useAddRecentTransaction();

    const { writeContract: mint, isPending: mintPending } = useWriteContract({
        mutation: {
            onSuccess: (tx_hash) => {
                addRecentTransaction({ hash: tx_hash, description: `minting name ${subname}` });
            },
            onSettled: () => {
                // arbitrary temporary sleep for tx to index in kimap_explorer app
                // can also listen to events on the frontend here, but tbh I'd rather put tx building completely to the backend
                // 
                setTimeout(() => {
                    refetchNode();
                }, 3500);
            },
        },
    });
    const { writeContract: note, isPending: notePending } = useWriteContract({
        mutation: {
            onSuccess: (tx_hash) => {
                addRecentTransaction({ hash: tx_hash, description: `adding note ${key}` });
            },
            onSettled: () => {
                setTimeout(() => {
                    refetchNode();
                }, 3500);
            },
        },
    });


    useEffect(() => {
        fetchInfo();
    }, [hash]);

    const fetchInfo = async () => {
        try {
            const data = await fetchNodeInfo(hash);
            setInfo(data);
        } catch (error) {
            console.error('Error fetching node info:', error);
        }
    };

    const handleAddNote = async () => {
        if (!address) {
            openConnectModal?.();
            return;
        }
        if (!info) return;
        const data = noteFunction(key, value);
        note({
            abi: mechAbi,
            functionName: 'execute',
            args: [
                KINOMAP,
                BigInt(0),
                data,
                0,
            ],
            address: info.tba as `0x${string}`,
        });

    };

    const handleMint = async () => {
        if (!address) {
            openConnectModal?.();
            return;
        }
        if (!info) return;
        const data = mintFunction(address, subname);
        mint({
            address: info.tba as `0x${string}`,
            abi: mechAbi,
            functionName: 'execute',
            args: [
                KINOMAP,
                BigInt(0),
                data,
                0,
            ],
        });

    };

    if (!info) return null;

    return (
        <div className="info-container">
            <div>Owner: {info.owner}</div>
            <div>TBA: {info.tba}</div>
            <div>Data Hex: {info.data_hex}</div>
            {info.owner.toLowerCase() === address?.toLowerCase() && (
                <div className="note-input-container">
                    <input type="text" placeholder="Key" value={key} onChange={(e) => setKey(e.target.value)} className="note-input" />
                    <input type="text" placeholder="Value" value={value} onChange={(e) => setValue(e.target.value)} className="note-input" />
                    <button
                        onClick={handleAddNote}
                        className={`add-note-button ${notePending ? 'loading' : ''}`}
                        disabled={notePending}
                    >
                        Add Note
                    </button>
                    <input type="text" placeholder="New Subname" value={subname} onChange={(e) => setSubname(e.target.value)} className="note-input" />
                    <button
                        onClick={handleMint}
                        className={`add-note-button ${mintPending ? 'loading' : ''}`}
                        disabled={mintPending}
                    >
                        Mint
                    </button>
                </div>
            )}
        </div>
    );

};

export default InfoContainer;