import React, { useState, useEffect } from 'react';
import InfoContainer from './InfoContainer';
import DataKeyElement from './DataKeyElement';
import { fetchNode } from '../abis/helpers';

interface NodeElementProps {
    hash: string;
}

interface Node {
    name: string;
    child_hashes: string[];
    data_keys: Record<string, string>;
}

const NodeElement: React.FC<NodeElementProps> = ({ hash }) => {
    const [node, setNode] = useState<Node | null>(null);
    const [expanded, setExpanded] = useState(false);
    const [infoVisible, setInfoVisible] = useState(false);

    useEffect(() => {
        fetchNodeData();
    }, [hash]);

    const fetchNodeData = async () => {
        try {
            const data = await fetchNode(hash);
            // console.log('fetched node', data);
            setNode(data);
        } catch (error) {
            console.error('Error fetching node:', error);
        }
    };

    if (!node) return null;

    const toggleExpanded = () => setExpanded(!expanded);
    const toggleInfo = () => setInfoVisible(!infoVisible);

    const hasChildren = node.child_hashes.length > 0 || Object.keys(node.data_keys).length > 0;

    return (
        <div className="node" data-hash={hash}>
            <div className="node-header" onClick={toggleExpanded}>
                {hasChildren && (
                    <span className={`arrow ${expanded ? 'expanded' : ''}`}></span>
                )}
                <span className="node-name">{node.name || 'Root'}</span>
                <span className="node-info">
                    ({Object.keys(node.data_keys).length}, {node.child_hashes.length})
                </span>
                <button className="info-button" onClick={(e) => { e.stopPropagation(); toggleInfo(); }}>ℹ️</button>
            </div>
            {infoVisible && <InfoContainer hash={hash} refetchNode={fetchNodeData} />}
            {expanded && hasChildren && (
                <div className="content">
                    <div className="child-nodes">
                        {node.child_hashes.map((childHash: string) => (
                            <NodeElement key={childHash} hash={childHash} />
                        ))}
                    </div>
                    <div className="data-keys">
                        {Object.entries(node.data_keys).map(([key, value]) => (
                            <DataKeyElement key={key} dataKey={key} dataValue={value as string} />
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
};

export default NodeElement;