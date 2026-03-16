"use client";

import { useState, useEffect } from "react";
import { useParams, useRouter } from "next/navigation";

const API_BASE = "http://localhost:3000/api/v1";

interface Project {
  name: string;
  owner_id: string;
  clone_url?: string;
}

interface User {
  id: string;
  name: string;
  email: string;
}

interface Permission {
  user_id: string;
  project: string;
  access: "read" | "write" | "admin";
}

export default function ProjectSettingsPage() {
  const params = useParams<{ projectName: string }>();
  const projectName = params.projectName;
  const router = useRouter();

  const [project, setProject] = useState<Project | null>(null);
  const [users, setUsers] = useState<User[]>([]);
  const [permissions, setPermissions] = useState<Permission[]>([]);
  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const [token, setToken] = useState("");

  // State for setting new permissions
  const [permUserId, setPermUserId] = useState("");
  const [permAccess, setPermAccess] = useState("read");

  useEffect(() => {
    const t = localStorage.getItem("token");
    if (!t) {
      // Redirect to login if not logged in
      router.push("/auth/login");
      return;
    }
    setToken(t);

    // Fetch current user details first
    fetch(`${API_BASE}/users/me`, {
      headers: { Authorization: `Bearer ${t}` },
    })
      .then((res) => {
        if (!res.ok) throw new Error("Auth failed");
        return res.json();
      })
      .then((data: User) => {
        setCurrentUser(data);
        fetchProjectAndPermissions(t, data.id);
      })
      .catch((error) => {
        console.error("Failed to fetch current user:", error);
        router.push("/auth/login"); // Redirect if auth fails
      });
  }, [projectName, router]);

  const fetchProjectAndPermissions = async (
    authToken: string,
    currentUserId: string,
  ) => {
    try {
      // Fetch project details
      const projectRes = await fetch(`${API_BASE}/projects/${projectName}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      if (!projectRes.ok) {
        if (projectRes.status === 404) {
          console.error("Project not found");
          router.push("/projects"); // Redirect if project not found
        } else {
          throw new Error(`Failed to fetch project: ${projectRes.statusText}`);
        }
        return;
      }
      const projectData: Project = await projectRes.json();
      const apiServerUrl = "http://localhost:3000";
      projectData.clone_url =
        projectData.clone_url || `${apiServerUrl}/${projectData.name}`;
      setProject(projectData);

      // Fetch all users and current permissions
      const [usersRes, permsRes] = await Promise.all([
        fetch(`${API_BASE}/users`, {
          headers: { Authorization: `Bearer ${authToken}` },
        }),
        fetch(`${API_BASE}/permissions`, {
          headers: { Authorization: `Bearer ${authToken}` },
        }),
      ]);
      const fetchedUsers = await usersRes.json();
      const fetchedPermissions = await permsRes.json();
      setUsers(fetchedUsers);
      setPermissions(fetchedPermissions);

      // Check if current user is admin for this project
      const userPermissions = fetchedPermissions.filter(
        (p: Permission) =>
          p.user_id === currentUserId && p.project === projectName,
      );
      const isAdminUser = userPermissions.some(
        (p: Permission) => p.access === "admin",
      );
      const isOwner = projectData.owner_id === currentUserId;
      setIsAdmin(isAdminUser || isOwner);

      if (!isAdminUser && !isOwner) {
        // Redirect if not an admin or owner
        console.warn("Unauthorized access to settings page");
        router.push(`/projects/${projectName}`); // Go back to project details
      }
    } catch (e) {
      console.error("Failed to fetch data", e);
      // Handle potential errors, e.g., redirect to login if token is invalid
      router.push("/auth/login");
    }
  };

  const handleSetPermission = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!token || !projectName || !permUserId || !permAccess) return;

    try {
      const res = await fetch(`${API_BASE}/permissions`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          user_id: permUserId,
          project: projectName,
          access: permAccess,
        }),
      });

      if (res.ok) {
        alert("Permission set successfully!");
        // Refresh permissions data
        const permsRes = await fetch(`${API_BASE}/permissions`, {
          headers: { Authorization: `Bearer ${token}` },
        });
        setPermissions(await permsRes.json());
        // Reset form
        setPermUserId("");
        setPermAccess("read");
      } else {
        const errorData = await res.json();
        alert(
          `Failed to set permission: ${errorData.message || res.statusText}`,
        );
      }
    } catch (error) {
      console.error("Error setting permission:", error);
      alert("An error occurred while setting permission.");
    }
  };

  // Filter users to show only those who are not the owner and not the current user for permission assignment
  const assignableUsers = users.filter(
    (user) => user.id !== project?.owner_id && user.id !== currentUser?.id,
  );

  if (!project || !currentUser || !isAdmin) {
    // Show loading or unauthorized message
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-50">
        <p className="text-lg text-gray-700">
          {isAdmin === false && !project
            ? "Loading project details..."
            : "Checking permissions..."}
        </p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="mx-auto max-w-4xl overflow-hidden rounded-lg bg-white shadow-md">
        <div className="p-6">
          <div className="mb-6 flex items-center justify-between">
            <h1 className="text-3xl font-bold text-gray-900">
              {project.name} Settings
            </h1>
            <a
              href={`/projects/${projectName}`}
              className="text-indigo-600 hover:text-indigo-500"
            >
              Back to Project
            </a>
          </div>

          <div className="mb-8">
            <h2 className="mb-4 text-2xl font-semibold text-gray-800">
              Manage User Permissions
            </h2>
            <div className="mb-6 overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                      User
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                      Email
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                      Access
                    </th>
                    {/* Add an 'Actions' column if needed for future features like revoking access */}
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200 bg-white">
                  {users
                    .filter(
                      (user) =>
                        user.id !== project.owner_id && // Don't show owner here
                        user.id !== currentUser.id && // Don't show current user here
                        permissions.some(
                          (p) =>
                            p.user_id === user.id && p.project === projectName,
                        ),
                    )
                    .map((user) => {
                      const userPerm = permissions.find(
                        (p) =>
                          p.user_id === user.id && p.project === projectName,
                      );
                      const access = userPerm
                        ? userPerm.access
                        : "No explicit access";
                      return (
                        <tr key={user.id}>
                          <td className="px-6 py-4 text-sm font-medium whitespace-nowrap text-gray-900">
                            {user.name}
                          </td>
                          <td className="px-6 py-4 text-sm whitespace-nowrap text-gray-500">
                            {user.email}
                          </td>
                          <td className="px-6 py-4 text-sm whitespace-nowrap text-gray-500 capitalize">
                            {access}
                          </td>
                        </tr>
                      );
                    })}
                </tbody>
              </table>
            </div>

            <h3 className="mt-6 mb-4 text-xl font-medium text-gray-900">
              Assign/Update Permission
            </h3>
            <form
              onSubmit={handleSetPermission}
              className="flex flex-wrap items-end gap-4"
            >
              <div className="min-w-[150px] flex-1">
                <label
                  htmlFor="user-select"
                  className="mb-1 block text-sm font-medium text-gray-700"
                >
                  Select User
                </label>
                <select
                  id="user-select"
                  value={permUserId}
                  onChange={(e) => setPermUserId(e.target.value)}
                  required
                  className="w-full rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none"
                >
                  <option value="">Select User</option>
                  {assignableUsers.map((u) => (
                    <option key={u.id} value={u.id}>
                      {u.name} ({u.email})
                    </option>
                  ))}
                </select>
              </div>
              <div className="min-w-[120px] flex-1">
                <label
                  htmlFor="access-select"
                  className="mb-1 block text-sm font-medium text-gray-700"
                >
                  Access Level
                </label>
                <select
                  id="access-select"
                  value={permAccess}
                  onChange={(e) => setPermAccess(e.target.value)}
                  className="w-full rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 focus:outline-none"
                >
                  <option value="read">Read</option>
                  <option value="write">Write</option>
                  <option value="admin">Admin</option>
                </select>
              </div>
              <button
                type="submit"
                className="rounded-md bg-green-600 px-4 py-2 text-white hover:bg-green-700 focus:ring-2 focus:ring-green-500 focus:ring-offset-2 focus:outline-none"
              >
                Set Permission
              </button>
            </form>
          </div>

          <div className="mt-12 border-t pt-6">
            <h2 className="mb-4 text-2xl font-semibold text-gray-800">
              Project Owner
            </h2>
            <p className="mb-2 text-lg text-gray-700">
              Owner: {project.owner_id}{" "}
              {/* This should ideally resolve to owner's name */}
            </p>
            {/* If current user is owner, you might show options to reassign ownership */}
          </div>
        </div>
      </div>
    </div>
  );
}
