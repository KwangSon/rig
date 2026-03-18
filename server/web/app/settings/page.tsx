"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";

const API_BASE = "http://localhost:3000/api/v1";

interface User {
  id: string;
  name: string;
  email: string;
}

interface Token {
  id: string;
  token_text: string;
  name: string | null;
  created_at: string;
  last_used_at: string | null;
}

export default function UserSettingsPage() {
  const router = useRouter();
  const [user, setUser] = useState<User | null>(null);
  const [tokens, setTokens] = useState<Token[]>([]);
  const [tokenName, setTokenName] = useState("");
  const [newTokenText, setNewTokenText] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const t = localStorage.getItem("token");
    if (!t) {
      router.push("/auth/login");
      return;
    }
    fetchData(t);
  }, [router]);

  const fetchData = async (authToken: string) => {
    setIsLoading(true);
    try {
      const [userRes, tokensRes] = await Promise.all([
        fetch(`${API_BASE}/users/me`, {
          headers: { Authorization: `Bearer ${authToken}` },
        }),
        fetch(`${API_BASE}/users/me/tokens`, {
          headers: { Authorization: `Bearer ${authToken}` },
        }),
      ]);

      if (userRes.ok && tokensRes.ok) {
        setUser(await userRes.json());
        setTokens(await tokensRes.json());
      } else {
        router.push("/auth/login");
      }
    } catch (e) {
      console.error(e);
    } finally {
      setIsLoading(false);
    }
  };

  const handleCreateToken = async (e: React.FormEvent) => {
    e.preventDefault();
    const t = localStorage.getItem("token");
    if (!t || !tokenName) return;

    try {
      const res = await fetch(`${API_BASE}/users/me/tokens`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${t}`,
        },
        body: JSON.stringify({ name: tokenName }),
      });

      if (res.ok) {
        const data = await res.json();
        setNewTokenText(data.token_text);
        setTokens([data, ...tokens]);
        setTokenName("");
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleDeleteToken = async (tokenId: string) => {
    if (!confirm("Are you sure you want to delete this token?")) return;
    const t = localStorage.getItem("token");
    if (!t) return;

    try {
      const res = await fetch(`${API_BASE}/users/me/tokens/${tokenId}`, {
        method: "DELETE",
        headers: { Authorization: `Bearer ${t}` },
      });

      if (res.ok) {
        setTokens(tokens.filter((tok) => tok.id !== tokenId));
      }
    } catch (e) {
      console.error(e);
    }
  };

  if (isLoading || !user) {
    return (
      <div className="flex min-h-[calc(100vh-56px)] items-center justify-center bg-gray-50">
        <div className="h-10 w-10 animate-spin rounded-full border-b-2 border-indigo-600"></div>
      </div>
    );
  }

  return (
    <div className="min-h-[calc(100vh-56px)] bg-gray-50 pb-12">
      <div className="mx-auto max-w-4xl px-4 py-10 sm:px-6 lg:px-8">
        <div className="mb-8 border-b border-gray-200 pb-5">
          <h1 className="text-3xl font-bold text-gray-900">Settings</h1>
          <p className="mt-2 text-sm text-gray-500">
            Manage your profile and security credentials.
          </p>
        </div>

        <div className="space-y-10">
          {/* Profile Section */}
          <section>
            <div className="mb-4 flex items-center gap-2">
              <svg
                className="h-5 w-5 text-gray-500"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
                />
              </svg>
              <h2 className="text-xl font-semibold text-gray-900">Profile</h2>
            </div>
            <div className="bg-white p-6 shadow ring-1 ring-gray-900/5 sm:rounded-lg">
              <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
                <div>
                  <label className="block text-sm font-medium text-gray-500">
                    Name
                  </label>
                  <p className="mt-1 text-base font-medium text-gray-900">
                    {user.name}
                  </p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-500">
                    Email
                  </label>
                  <p className="mt-1 text-base font-medium text-gray-900">
                    {user.email}
                  </p>
                </div>
              </div>
            </div>
          </section>

          {/* Personal Access Tokens Section */}
          <section>
            <div className="mb-4 flex items-center gap-2">
              <svg
                className="h-5 w-5 text-gray-500"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"
                />
              </svg>
              <h2 className="text-xl font-semibold text-gray-900">
                Personal Access Tokens
              </h2>
            </div>
            <div className="bg-white shadow ring-1 ring-gray-900/5 sm:rounded-lg">
              <div className="p-6">
                <p className="mb-6 text-sm text-gray-600">
                  Tokens you have generated that can be used to access the Rig
                  API via CLI.
                </p>

                {newTokenText && (
                  <div className="mb-6 rounded-md border border-green-200 bg-green-50 p-4">
                    <div className="flex">
                      <div className="flex-shrink-0">
                        <svg
                          className="h-5 w-5 text-green-400"
                          viewBox="0 0 20 20"
                          fill="currentColor"
                        >
                          <path
                            fillRule="evenodd"
                            d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                            clipRule="evenodd"
                          />
                        </svg>
                      </div>
                      <div className="ml-3 flex-1">
                        <h3 className="text-sm font-medium text-green-800">
                          New token generated
                        </h3>
                        <div className="mt-2 text-sm text-green-700">
                          <p>
                            Make sure to copy your new personal access token
                            now. You won't be able to see it again!
                          </p>
                          <div className="mt-3 flex items-center gap-2">
                            <code className="rounded border border-green-200 bg-white px-3 py-1.5 font-mono text-base font-bold text-gray-900 select-all">
                              {newTokenText}
                            </code>
                            <button
                              onClick={() =>
                                navigator.clipboard.writeText(newTokenText)
                              }
                              className="rounded bg-green-100 px-2 py-1 text-xs font-semibold text-green-800 hover:bg-green-200"
                            >
                              Copy
                            </button>
                          </div>
                        </div>
                      </div>
                      <div className="ml-auto pl-3">
                        <div className="-mx-1.5 -my-1.5">
                          <button
                            onClick={() => setNewTokenText(null)}
                            className="inline-flex rounded-md p-1.5 text-green-500 hover:bg-green-100 focus:ring-2 focus:ring-green-600 focus:ring-offset-2 focus:outline-none"
                          >
                            <span className="sr-only">Dismiss</span>
                            <svg
                              className="h-5 w-5"
                              viewBox="0 0 20 20"
                              fill="currentColor"
                            >
                              <path
                                fillRule="evenodd"
                                d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                                clipRule="evenodd"
                              />
                            </svg>
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                )}

                <form
                  onSubmit={handleCreateToken}
                  className="mb-8 flex items-end gap-4"
                >
                  <div className="flex-1">
                    <label
                      htmlFor="token-name"
                      className="mb-1 block text-sm font-medium text-gray-700"
                    >
                      Token Name
                    </label>
                    <input
                      type="text"
                      id="token-name"
                      placeholder="e.g. My MacBook Air"
                      value={tokenName}
                      onChange={(e) => setTokenName(e.target.value)}
                      required
                      className="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-gray-300 ring-inset placeholder:text-gray-400 focus:ring-2 focus:ring-indigo-600 focus:ring-inset sm:text-sm sm:leading-6"
                    />
                  </div>
                  <button
                    type="submit"
                    className="inline-flex justify-center rounded-md bg-indigo-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                  >
                    Generate Token
                  </button>
                </form>

                <div className="overflow-x-auto border-t border-gray-100">
                  <table className="min-w-full divide-y divide-gray-200">
                    <thead>
                      <tr>
                        <th className="px-3 py-3.5 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                          Name
                        </th>
                        <th className="px-3 py-3.5 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                          Created
                        </th>
                        <th className="px-3 py-3.5 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                          Last Used
                        </th>
                        <th className="relative py-3.5 pr-4 pl-3 sm:pr-6">
                          <span className="sr-only">Actions</span>
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-100 bg-white text-sm">
                      {tokens.map((tok) => (
                        <tr
                          key={tok.id}
                          className="transition-colors hover:bg-gray-50"
                        >
                          <td className="px-3 py-4 font-medium whitespace-nowrap text-gray-900">
                            {tok.name || "Untitled"}
                          </td>
                          <td className="px-3 py-4 whitespace-nowrap text-gray-500">
                            {new Date(tok.created_at).toLocaleDateString()}
                          </td>
                          <td className="px-3 py-4 whitespace-nowrap text-gray-500">
                            {tok.last_used_at
                              ? new Date(tok.last_used_at).toLocaleString()
                              : "Never used"}
                          </td>
                          <td className="py-4 pr-4 pl-3 text-right whitespace-nowrap sm:pr-6">
                            <button
                              onClick={() => handleDeleteToken(tok.id)}
                              className="font-medium text-red-600 hover:text-red-900"
                            >
                              Revoke
                            </button>
                          </td>
                        </tr>
                      ))}
                      {tokens.length === 0 && (
                        <tr>
                          <td
                            colSpan={4}
                            className="px-3 py-8 text-center text-gray-500 italic"
                          >
                            No personal access tokens generated yet.
                          </td>
                        </tr>
                      )}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
