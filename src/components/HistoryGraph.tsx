import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect } from "react";
import "./HistoryGraph.css";

interface HistoryNode {
    id: string;
    parent_id: string | null;
    action_name: string;
    timestamp: number;
    is_current: boolean;
    is_branch_head: boolean;
}

export const HistoryGraph = () => {
    const [nodes, setNodes] = useState<HistoryNode[]>([]);
    const [branches, setBranches] = useState<Record<string, string>>({});

    const fetchHistory = async () => {
        try {
            const graph = await invoke<Array<[string, string | null, string]>>("get_history_graph");
            const current = await invoke<string>("get_current_node");
            const branchData = await invoke<Record<string, string>>("get_branches");

            const branchHeads = new Set(Object.values(branchData));

            const historyNodes: HistoryNode[] = graph.map(([id, parent, action]: [string, string | null, string]) => ({
                id,
                parent_id: parent,
                action_name: action,
                timestamp: 0,
                is_current: id === current,
                is_branch_head: branchHeads.has(id),
            }));

            setNodes(historyNodes);
            setBranches(branchData);
        } catch (e) {
            console.error("Failed to fetch history:", e);
        }
    };

    useEffect(() => {
        fetchHistory();
        const interval = setInterval(fetchHistory, 1000);
        return () => clearInterval(interval);
    }, []);

    const handleCheckout = async (node_id: string) => {
        try {
            await invoke("checkout_node", { nodeId: node_id });
            fetchHistory();
        } catch (e) {
            console.error("Checkout failed:", e);
        }
    };

    const handleUndo = async () => {
        try {
            await invoke("undo");
            fetchHistory();
        } catch (e) {
            console.error("Undo failed:", e);
        }
    };

    const handleRedo = async () => {
        try {
            await invoke("redo");
            fetchHistory();
        } catch (e) {
            console.error("Redo failed:", e);
        }
    };

    const handleCreateBranch = async () => {
        const branchName = prompt("Enter branch name:");
        if (branchName) {
            try {
                await invoke("create_branch", { branchName });
                fetchHistory();
            } catch (e) {
                console.error("Branch creation failed:", e);
            }
        }
    };

    // Simple vertical layout (can be enhanced with D3.js for complex DAG visualization)
    return (
        <div className="history-graph glass">
            <div className="history-controls">
                <button className="btn-history" onClick={handleUndo}>
                    ← Undo
                </button>
                <button className="btn-history" onClick={handleRedo}>
                    Redo →
                </button>
                <button className="btn-history" onClick={handleCreateBranch}>
                    + Branch
                </button>
            </div>

            <div className="history-timeline">
                {nodes.map((node: HistoryNode) => (
                    <div
                        key={node.id}
                        className={`history-node ${node.is_current ? 'current' : ''} ${node.is_branch_head ? 'branch-head' : ''}`}
                        onClick={() => handleCheckout(node.id)}
                    >
                        <div className="node-indicator" />
                        <div className="node-info">
                            <span className="node-action">{node.action_name}</span>
                            <span className="node-id-sub">{node.id.slice(0, 8)}</span>
                            {node.is_current && <span className="node-badge">HEAD</span>}
                            {node.is_branch_head && (
                                <span className="node-badge branch">
                                    {Object.entries(branches).find(([_, id]) => id === node.id)?.[0]}
                                </span>
                            )}
                        </div>
                    </div>
                ))}
            </div>

            <div className="branch-list">
                <h3>Branches</h3>
                {Object.entries(branches).map(([name, nodeId]: [string, string]) => (
                    <div key={name} className="branch-item">
                        <span className="branch-name">{name}</span>
                        <span className="branch-commit">{nodeId.slice(0, 8)}</span>
                    </div>
                ))}
            </div>
        </div>
    );
};
