export type Proto = "ssh" | "serial";
export type Auth = "password" | "key";

export interface Host {
  id: string;
  name: string;
  proto: Proto;
  host?: string;
  port?: number;
  username?: string;
  auth?: Auth;
  keyPath?: string;
  hasPassword?: boolean;
  hasKey?: boolean;
  serialPort?: string;
  baud?: number;
}

export interface HostInput {
  id?: string;
  name: string;
  proto: Proto;
  host?: string;
  port?: number;
  username?: string;
  auth?: Auth;
  password?: string;
  keyPath?: string;
  passphrase?: string;
  serialPort?: string;
  baud?: number;
}
