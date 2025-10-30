I'd like to enhance this project in a few ways. First, I'd like to create a docker version of maproom-mcp, and I'd like to have the container include a sub container for running a local LLM model for doing the embeddings, and have that be the default, so that it works without any API keys or access to the internet. That would also let the target architecture for maproom to be more easily portable to other machines. I'd like you to create a new project in crewchief_context/maproom with a slug of LOCAL in a folder called LOCAL. The project folder should have an LOCAL_ANALYSIS, LOCAL_ARCHITECTURE, and LOCAL_PLAN document. For LOCAL_ANALYSIS, Start with analysis of the problem space. Look at local LLM options and their pros and cons and ultimately with a focus on performance, but also considering things like download size and memory usage. For LOCAL_ARCHITECTURE, outline the technical details of the project, and the architecture of the project. For LOCAL_PLAN, outline the steps to complete the project, broken down into phases and deliverables, as well as the agents that will be best suited to complete the various parts of the project. If you identify a gap in available agents, create a new agent definition in .agents/specialized-agents. Ideally the end user shouldn't have to worry about configuration beyond a single docker command in the mcp config to run maproom-mcp and have it include the local LLM model. The container should also include postgress, so that doesn't have to be set up separately. During analysis, seek a solution that means the minimum amount of configuration for the end user -- as plug-and-play as possible. Don't commit this work until I've had a chance to review it. 










