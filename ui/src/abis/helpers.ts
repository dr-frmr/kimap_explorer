import { encodeFunctionData, encodePacked, stringToHex } from "viem";
import { kinomapAbi, KINO_ACCOUNT_IMPL } from "./";

// GETs to kinode app
export async function fetchNode(hash: string) {
    const response = await fetch(`/kimap_explorer:kimap_explorer:pacificat.os/api/node/${hash}`);
    return await response.json();
}

export async function fetchNodeInfo(hash: string) {
    const response = await fetch(`/kimap_explorer:kimap_explorer:pacificat.os/api/info/${hash}`);
    return await response.json();
}


// chain interaction encoding functions
export function mintFunction(our_address: `0x${string}`, nodename: string) {
    return encodeFunctionData({
        abi: kinomapAbi,
        functionName: 'mint',
        args: [
            our_address,
            encodePacked(["bytes"], [stringToHex(nodename)]),
            "0x", // empty initial calldata
            "0x", // empty erc721 details
            KINO_ACCOUNT_IMPL,
        ]
    })
}

export function noteFunction(key: string, value: string) {
    return encodeFunctionData({
        abi: kinomapAbi,
        functionName: 'note',
        args: [
            encodePacked(["bytes"], [stringToHex(key)]),
            encodePacked(["bytes"], [stringToHex(value)]),
        ]
    });
}