import React from 'react';
import NodeElement from './NodeElement';

interface TreeContainerProps {
    node: any;
}

const TreeContainer: React.FC<TreeContainerProps> = ({ node }) => {
    return (
        <div className="tree-container">
            {node.child_hashes.map((hash: string) => (
                <NodeElement key={hash} hash={hash} />
            ))}
        </div>
    );
};

export default TreeContainer;