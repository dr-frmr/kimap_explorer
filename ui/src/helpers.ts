import { encodeFunctionData, encodePacked, stringToHex } from "viem";
import { kinomapAbi, KINO_ACCOUNT_IMPL, API_PATH } from "./abis";

// GETs to kinode app

export async function fetchNode(name: string) {
    const response = await fetch(API_PATH, {
        method: 'POST',
        body: JSON.stringify({
            GetNode: name
        })
    });
    return await response.json();
}

export async function fetchNodeInfo(name: string) {
    const response = await fetch(API_PATH, {
        method: 'POST',
        body: JSON.stringify({
            GetTba: name
        })
    });
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

export function factFunction(key: string, value: string) {
    return encodeFunctionData({
        abi: kinomapAbi,
        functionName: 'fact',
        args: [
            encodePacked(["bytes"], [stringToHex(key)]),
            encodePacked(["bytes"], [stringToHex(value)]),
        ]
    });
}