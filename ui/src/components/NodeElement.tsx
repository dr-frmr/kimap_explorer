import React, { useState, useEffect } from 'react';
import InfoContainer from './InfoContainer';
import { DataKeyElement, DataKey } from './DataKeyElement';
import { fetchNode } from '../helpers';

interface NodeElementProps {
    hash: string;
}

interface Node {
    name: string;
    parent_path: string;
    child_hashes: string[];
    data_keys: Record<string, DataKey>;
}

const NodeElement: React.FC<NodeElementProps> = ({ hash }) => {
    const [node, setNode] = useState<Node | null>(null);
    const [expanded, setExpanded] = useState(false);
    const [infoVisible, setInfoVisible] = useState(false);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        fetchNodeData();
    }, [hash]);

    const fetchNodeData = async () => {
        try {
            const data = await fetchNode(hash);
            setNode(data);
        } catch (error) {
            console.error('Error fetching node:', error);
        } finally {
            setLoading(false);
        }
    };

    const toggleExpanded = () => setExpanded(!expanded);
    const toggleInfo = () => setInfoVisible(!infoVisible);

    // Show a minimal placeholder while loading
    if (loading && !node) {
        return (
            <div className="node" data-hash={hash}>
                <div className="node-header">
                    <span className="node-name">Loading...</span>
                </div>
            </div>
        );
    }

    // After loading, if no node was found
    if (!node) return null;

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
                        {Object.entries(node.data_keys).map(([label, data_key]) => (
                            <DataKeyElement key={label} dataKey={label} dataValue={data_key} />
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
};

export default NodeElement;