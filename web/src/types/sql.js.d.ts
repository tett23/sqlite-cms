// sql.js は型を同梱していないので、このリポジトリで使う API だけを宣言する。
declare module "sql.js" {
  export type SqlValue = number | string | Uint8Array | null;
  export type BindParams = SqlValue[] | Record<string, SqlValue> | null;

  export interface QueryExecResult {
    columns: string[];
    values: SqlValue[][];
  }

  export interface Statement {
    bind(values?: BindParams): boolean;
    step(): boolean;
    get(params?: BindParams): SqlValue[];
    free(): boolean;
  }

  export interface Database {
    exec(sql: string, params?: BindParams): QueryExecResult[];
    run(sql: string, params?: BindParams): Database;
    prepare(sql: string, params?: BindParams): Statement;
    close(): void;
  }

  export interface SqlJsStatic {
    Database: new (data?: ArrayLike<number> | null) => Database;
  }

  export interface InitSqlJsConfig {
    locateFile?: (file: string) => string;
  }

  export default function initSqlJs(config?: InitSqlJsConfig): Promise<SqlJsStatic>;
}
