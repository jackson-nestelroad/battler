import * as crypto from 'crypto';
import * as argon2 from 'argon2';

function escapeId(s: string): string {
    return s.replace(/=/g, '=3D').replace(/,/g, '=2C');
}

export async function saltPassword(
    password: string,
    saltB64: string,
    kdf: string,
    iterations: number,
    memory?: number
): Promise<string> {
    const saltBytes = Buffer.from(saltB64, 'base64');
    const cleanPassword = password.normalize('NFC'); // saslprep equivalent for basic use

    if (kdf === 'pbkdf2') {
        const rounds = iterations;
        const keylen = 32;
        const hash = crypto.pbkdf2Sync(cleanPassword, saltBytes, rounds, keylen, 'sha256');
        const cleanB64 = (buf: Buffer) => buf.toString('base64').replace(/=/g, '');
        return `$pbkdf2-sha256$i=${rounds}$${cleanB64(saltBytes)}$${cleanB64(hash)}`;
    } else if (kdf === 'argon2id13') {
        // argon2 npm package expects memory in KB. 
        // In Rust password-hash, memory is in KB (e.g. 4096 = 4MB).
        const phc = await argon2.hash(cleanPassword, {
            type: argon2.argon2id,
            memoryCost: memory ?? 4096,
            timeCost: iterations,
            parallelism: 1,
            salt: saltBytes,
            raw: false,
        });
        return phc;
    } else {
        throw new Error(`Unsupported KDF: ${kdf}`);
    }
}

export function computeHMAC(key: Buffer | string, data: string | Buffer): Buffer {
    const hmac = crypto.createHmac('sha256', key);
    hmac.update(data);
    return hmac.digest();
}

export function computeSHA256(data: Buffer): Buffer {
    const hash = crypto.createHash('sha256');
    hash.update(data);
    return hash.digest();
}

export function xorBuffers(a: Buffer, b: Buffer): Buffer {
    const res = Buffer.alloc(a.length);
    for (let i = 0; i < a.length; i++) {
        res[i] = a[i] ^ b[i];
    }
    return res;
}

export interface ScramChallenge {
    nonce: string;
    salt: string;
    kdf: string;
    iterations: number;
    memory?: number;
}

export interface ScramResponse {
    signature: string; // Base64 client proof
    expectedServerSignature: string; // Base64 server signature to verify welcome
}

export async function generateScramResponse(
    authid: string,
    password: string,
    clientNonce: string,
    challenge: ScramChallenge
): Promise<ScramResponse> {
    // 1. Salt the password
    const saltedPasswordStr = await saltPassword(
        password,
        challenge.salt,
        challenge.kdf,
        challenge.iterations,
        challenge.memory
    );
    const saltedPassword = Buffer.from(saltedPasswordStr, 'utf8');

    // 2. Derive Client Key, Stored Key, and Server Key
    const clientKey = computeHMAC(saltedPassword, 'Client Key');
    const storedKey = computeSHA256(clientKey);
    const serverKey = computeHMAC(saltedPassword, 'Server Key');

    // 3. Construct Auth Message
    // client_first_bare = n=<authid>,r=<client_nonce>
    const clientFirstBare = `n=${escapeId(authid)},r=${clientNonce}`;
    // server_first = r=<server_nonce>,s=<salt>,i=<iterations> (server_nonce here is server random part only)
    const serverNonceOnly = challenge.nonce.startsWith(clientNonce)
        ? challenge.nonce.slice(clientNonce.length)
        : challenge.nonce;
    const serverFirst = `r=${serverNonceOnly},s=${challenge.salt},i=${challenge.iterations}`;
    // client_final_no_proof = c=biws,r=<challenge_nonce> (biws is b64 of 'n,,')
    const clientFinalNoProof = `c=biws,r=${challenge.nonce}`;

    const authMessage = `${clientFirstBare},${serverFirst},${clientFinalNoProof}`;

    // 4. Compute Client Signature and Client Proof
    const clientSignature = computeHMAC(storedKey, authMessage);
    const clientProof = xorBuffers(clientKey, clientSignature);

    // 5. Compute Expected Server Signature
    const serverSig = computeHMAC(serverKey, authMessage);

    return {
        signature: clientProof.toString('base64'),
        expectedServerSignature: serverSig.toString('base64'),
    };
}
