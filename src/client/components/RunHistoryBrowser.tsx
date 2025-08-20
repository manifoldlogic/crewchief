import React from "react";
import { useQuery } from "@apollo/client";
import { GET_RUNS } from "../queries/runQueries";

const RunHistoryBrowser = () => {
  const { data } = useQuery(GET_RUNS);

  // List, timeline, log, export

  return <div>Run History</div>;
};

export default RunHistoryBrowser;
