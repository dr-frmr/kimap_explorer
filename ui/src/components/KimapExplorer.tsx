import React, { useState, useEffect } from 'react';
import TreeContainer from './TreeContainer';
import { fetchNode } from '../helpers';

const KimapExplorer: React.FC = () => {
    const [rootNode, setRootNode] = useState(null);

    useEffect(() => {
        fetchRootNode();
    }, []);

    const fetchRootNode = async () => {
        try {
            const data = await fetchNode('0x0000000000000000000000000000000000000000000000000000000000000000');
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