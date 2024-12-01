import axios from "axios";

async function callServer(url: string): Promise<any> {
    try {
        const response = await axios.post<any>(url);
        return response.data;
    } catch (error) {
        if (axios.isAxiosError(error)) {
            throw new Error(`API Error: ${error.message}`);
        }
        throw error;
    }
}

// The following functions are not used in production. 
// They are used to explicitly change the state of the test server.
export async function postEmptyBlock(block_builder_base_url: string,): Promise<void> {
    const url = `${block_builder_base_url}/block-builder/post-empty-block`;
    await callServer(url);
}