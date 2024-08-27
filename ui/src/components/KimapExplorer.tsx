import React, { useState, useEffect } from 'react';
import TreeContainer from './TreeContainer';

const KimapExplorer: React.FC = () => {
    const [rootNode, setRootNode] = useState(null);

    useEffect(() => {
        fetchRootNode();
    }, []);

    const fetchRootNode = async () => {
        try {
            const response = await fetch('/kimap_explorer:kimap_explorer:doria.kino/api/node/0x0000000000000000000000000000000000000000000000000000000000000000');
            const data = await response.json();
            setRootNode(data);
        } catch (error) {
            console.error('Error fetching root node:', error);
        }
    };

    return (
        <div className="explorer-container">
            <div className="explorer-explanation">(data-keys, children)</div>
            {rootNode && <TreeContainer node={rootNode} />}
        </div>
    );
};

export default KimapExplorer;