Now create tickets for projects definied in crewchief_context/maproom. Use the MAPROOM_PROJECT_OVERVIEW.md file to get an overview of the projects. Tickets should be created in .agents/work-tickets. Tickets should be individual files named like PROJ-1001_ticket-name.md where PROJ is the project slug (like LANG_PARSE) and the ticket number is the next sequential number. Phases should be the first number prepended to the ticket number. e.g. HYBRID_SEARCH-1001_ticket-name.md would be the first ticket in the first phase of the HYBRID_SEARCH project. Ticets should be created by the create-ticket agent, who should be provided the project slug, ticket number, and enough context to write the ticket well, so that the implementing agent can follow the ticket and complete the work. The create-ticket agent should also be provided relevant docs to link to in the ticket, as well as the agents that should be used to implement the ticket. Evaluate where tests would be useful to add, based on this being an MVP, and we are not looking for full coverage, and just the tests that will truly help our velocity, by providing confidence. Test creation should be independent tickets.




Given the current agents we've created and the work they will be doing, what different type of agents for writing tests would be useful? Base this on this being an MVP, where we are not looking for full coverage, and just the tests that will truly help our velocity, by providing confidence. We don't want to be slowed down in situations where having tests would've saved us time, though we didn't have them. But we also don't want to spend too much time writing tests just for their own sake. See crewchief_context/maproom/MAPROOM_PROJECT_OVERVIEW.md to understand the overall project, to help you decide which test creation agents would be useful. Think about what types of tests we will want and what types of agents would be useful to write them. Create a document similar to crewchief_context/maproom/AGENT_RECOMMENDATIONS.md.







