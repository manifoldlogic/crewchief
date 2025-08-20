import { FixedSizeList as List } from "react-window";

const LogViewer = ({ logs }) => {
  return (
    <List height={400} itemCount={logs.length} itemSize={20} width={600}>
      {({ index, style }) => <div style={style}>{logs[index]}</div>}
    </List>
  );
};
