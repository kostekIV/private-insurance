import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { config } from "../../config";

export const expressionApi = createApi({
  reducerPath: "expressionApi",
  baseQuery: fetchBaseQuery({ baseUrl: config.baseUrl }),
  endpoints: (builder) => ({
    expr: builder.query<{ msg: string }, any>({
      query: (body) => ({
        url: "exp",
        method: "POST",
        body,
      }),
    }),
  }),
});

export const { useExprQuery, useLazyExprQuery } = expressionApi;
